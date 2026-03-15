use std::env;

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