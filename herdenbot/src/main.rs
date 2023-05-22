use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::process::Command;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let secret_file = "/secrets/herdenbot/bot.json";
    log::info!("Starting command bot...");
    let data = std::fs::read_to_string(secret_file)
        .expect(&format!("Unable to read file {}", secret_file));
    let data: serde_json::Value = serde_json::from_str(&data).expect("JSON was not well-formatted");
    let token = data["token"].as_str().expect("Token was not a string");
    log::info!("Starting throw dice bot...");

    let bot = Bot::new(token);

    BotCommand::repl(bot, answer).await;

    /* teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    })
    .await; */
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum BotCommand {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "start the valheim server")]
    StartValheim,
    #[command(description = "stop the valheim server")]
    StopValheim,
}

async fn answer(bot: Bot, msg: Message, cmd: BotCommand) -> ResponseResult<()> {
    log::info!("Received {:?}", msg);
    match cmd {
        BotCommand::Help => {
            bot.send_message(msg.chat.id, BotCommand::descriptions().to_string())
                .await?
        }
        BotCommand::StartValheim => {
            bot.send_message(msg.chat.id, format!("Starting valheim..."))
                .await?;
            let output = Command::new("/run/wrappers/bin/sudo")
                .arg("/run/current-system/sw/bin/systemctl")
                .arg("start")
                .arg("valheim-server.service")
                .output()
                .await?;

            if output.status.success() {
                //write current timestamp to /home/herdenbot/last_start.timestamp
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();
                std::fs::write("/home/herdenbot/last_start.timestamp", timestamp.to_string())?;
                bot.send_message(msg.chat.id, format!("success")).await?
            } else {
                let error_message = String::from_utf8_lossy(&output.stderr);

                bot.send_message(msg.chat.id, format!("error {}", error_message))
                    .await?
            }
        }
        BotCommand::StopValheim => {
            bot.send_message(msg.chat.id, format!("Stopping valheim"))
                .await?;
            let output = Command::new("/run/wrappers/bin/sudo")
                .arg("/run/current-system/sw/bin/systemctl")
                .arg("stop")
                .arg("valheim-server.service")
                .output()
                .await?;

            if output.status.success() {
                bot.send_message(msg.chat.id, format!("success")).await?
            } else {
                let error_message = String::from_utf8_lossy(&output.stderr);

                bot.send_message(msg.chat.id, format!("error {}", error_message))
                    .await?
            }

        }
    };

    Ok(())
}
