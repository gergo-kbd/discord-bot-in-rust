/*use std::env;

use serenity::all::{EventHandler, GatewayIntents};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use rand::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        
        if msg.author.bot { return; }

        let content = msg.content.trim();
        
        let mut parts = content.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match msg.content.as_str(){
            "!ping" => { msg.channel_id.say(&ctx.http, "Pong!").await; }
            c if c.starts_with("!roll") => {  

                let max_roll = parts.next()
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(100);

                let response = {
                    let mut rng = rand::rng();
                    let roll = rng.random_range(0..=max_roll);
                    format!( "{} rolled: *{}* (0-{})", msg.author.name, roll, max_roll)
                };

                if let Err(why) = msg.channel_id.say(&ctx.http, response).await{
                    println!("Error sending message: {why:?}");
                }
            }
            "!help" => { msg.channel_id.say(&ctx.http, "commands: !ping, !roll, !roll [n]").await; }
            _ => (),
        }
        
    }
}

#[tokio::main]
async fn main() {
    // bot token login
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    
    let mut client = 
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }

}
*/

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use sqlx::sqlite::SqlitePool;
use std::env;

use yahoo_finance_api as yf;

struct Handler {
    db: sqlx::SqlitePool,
    api_key: String,
}

#[async_trait]
impl EventHandler for Handler {
   async fn message(&self, ctx: Context, msg: Message) {
    // Csak a parancsokra figyeljen
    if msg.content.starts_with("!analyze ") {
        println!("Analízis kérés érkezett: {}", msg.content);

        let ticker = msg.content.replace("!analyze ", "").trim().to_uppercase();
        
        if ticker.is_empty() {
             let _ = msg.channel_id.say(&ctx.http, "Adj meg egy szimbólumot! (Pl: !analyze AAPL)").await;
             return;
        }

        // 1. Prompt lekérése
        let row: (String,) = sqlx::query_as("SELECT content FROM prompts WHERE name = 'master'")
            .fetch_one(&self.db)
            .await
            .unwrap_or(("Te egy tőzsdei elemző vagy.".to_string(),));

        // 2. Gemini Hívás
        let _ = msg.channel_id.broadcast_typing(&ctx.http).await; 
        let analysis = self.call_gemini(&row.0, &ticker).await;

        // 3. Biztonságos küldés (darabolás, ha túl hosszú)
        let mut current_text = analysis.as_str();
        
        while !current_text.is_empty() {
            // Ha hosszabb mint 1900 karakter, keressünk egy biztonságos töréspontot
            if current_text.len() > 1900 {
                let wrap_point = current_text[..1900].rfind('\n').unwrap_or(1900);
                let chunk = &current_text[..wrap_point];
                let _ = msg.channel_id.say(&ctx.http, chunk).await;
                current_text = current_text[wrap_point..].trim_start();
            } else {
                // Ha belefér, küldjük el az egészet
                if let Err(why) = msg.channel_id.say(&ctx.http, current_text).await {
                    println!("Hiba az üzenet küldésekor: {:?}", why);
                }
                break;
            }
        }
    }
        if msg.content.starts_with("!setprompt ") {
            let new_prompt = msg.content.trim_start_matches("!setprompt ").trim();
            
            if new_prompt.is_empty() {
                // Itt kivettem a ?-et a végéről
                let _ = msg.channel_id.say(&ctx.http, "Hiba: Adj meg egy új prompt szöveget!").await;
            } else {
                let result = sqlx::query("UPDATE prompts SET content = ? WHERE name = 'master'")
                    .bind(new_prompt)
                    .execute(&self.db)
                    .await;

                match result {
                    Ok(_) => {
                        // Itt is kivettem a ?-et
                        let _ = msg.channel_id.say(&ctx.http, "✅ A Master Prompt sikeresen frissítve!").await;
                    },
                    Err(e) => {
                        // És itt is
                        let _ = msg.channel_id.say(&ctx.http, format!("❌ Adatbázis hiba: {}", e)).await;
                    }
                }
            }
        }

    }
}

impl Handler {
async fn call_gemini(&self, system_prompt: &str, user_data: &str) -> String {
    let client = reqwest::Client::new();
    // A listád alapján a gemini-2.0-flash biztosan létezik nálad
    let url = format!("https://generativelanguage.googleapis.com/v1/models/gemini-2.5-flash:generateContent?key={}", self.api_key);


    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {"text": format!("SYSTEM: {}\n\nUSER DATA: {}", system_prompt, user_data)}
            ]
        }]
    });

    let res = client.post(url).json(&payload).send().await;
    match res {
        Ok(response) => {
            let status = response.status();
            let json: serde_json::Value = response.json().await.unwrap_or_default();
            
            if !status.is_success() {
                println!("Gemini API Hiba ({}): {:?}", status, json);
                return format!("API Hiba: {}. Ellenőrizd a fly logs-ot!", status);
            }

            json["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Az AI nem küldött választ.".to_string())
        },
        Err(e) => format!("Hálózati hiba: {:?}", e),
    }
}
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN hiányzik");
    let gemini_key = env::var("GEMINI_KEY").expect("GEMINI_KEY hiányzik");
    
    // 1. Útvonal meghatározása (Fly.io vs Local)
    let is_fly = std::path::Path::new("/data").exists();
    
    // FONTOS: Fly-on a három perjel (sqlite:///) jelzi az abszolút utat Linuxon!
    let database_url = if is_fly {
        "sqlite:///data/bot.db".to_string()
    } else {
        "sqlite:bot.db".to_string()
    };

    // 2. Mappa és jogosultságok előkészítése Fly.io-n
    if is_fly {
        println!("Fly.io környezet: /data mappa ellenőrzése...");
        let _ = std::fs::create_dir_all("/data");
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions("/data", std::fs::Permissions::from_mode(0o777));
            println!("Jogosultságok beállítva (0777).");
        }
    }

    println!("Adatbázis csatlakozás: {}", database_url);

    // 3. Adatbázis inicializálása speciális opciókkal
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let opts = SqliteConnectOptions::from_str(&database_url)
        .expect("Hibás adatbázis URL formátum")
        .create_if_missing(true); // Automatikusan létrehozza a bot.db-t, ha nincs ott

    let db = sqlx::SqlitePool::connect_with(opts)
        .await
        .expect("Nem sikerült kapcsolódni az adatbázishoz (Code 14?)");

    // Tábla létrehozása
    sqlx::query("CREATE TABLE IF NOT EXISTS prompts (name TEXT PRIMARY KEY, content TEXT)")
        .execute(&db)
        .await
        .expect("Hiba a tábla létrehozásakor");

    // Alapértelmezett prompt beszúrása
    let _ = sqlx::query("INSERT OR IGNORE INTO prompts (name, content) VALUES ('master', 'Alapértelmezett elemző vagy.')")
        .execute(&db)
        .await;

    // 4. Discord kliens indítása
    let mut client = Client::builder(&token, GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT)
        .event_handler(Handler { 
            db, 
            api_key: gemini_key 
        })
        .await
        .expect("Hiba a Discord kliens létrehozásakor");

    println!("A bot sikeresen elindult és csatlakozott!");

    if let Err(why) = client.start().await {
        println!("Kliens hiba a futás során: {:?}", why);
    }
}