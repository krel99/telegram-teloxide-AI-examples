use dotenv::dotenv;
use genai::chat::{ChatMessage, ChatRequest};
use genai::Client;
use serde_json::json;
use std::env;
use teloxide::types::InputFile;

use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

const MODEL_OPENAI: &str = "gpt-4o-mini";
const OPENAI_ENV_NAME: &str = "OPENAI_API_KEY";

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

// Default - instance creator
// Debug - allows me to log State to console
// This is not a real state management as it stores only the last question!
#[derive(Clone, Default)] // why clone necessary, what is default, when is debug used
pub enum State {
    #[default]
    Start,
    Question {
        full_question: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    log::info!("Starting bot...");
    dotenv().ok(); // creates env variables from env

    let bot = Bot::from_env();

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(dptree::case![State::Start].endpoint(start)),
    )
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    Ok(())
}

async fn start(bot: Bot, msg: Message) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            let eleven_labs_key = env::var("ELEVENLABS_API_KEY")
                .expect("ELEVENLABS_API_KEY environment variable not set");
            let voice_id =
                env::var("ELEVEN_VOICE_ID").expect("ELEVEN_VOICE_ID environment variable not set");
            let chat_req = ChatRequest::new(vec![
                ChatMessage::system("Answer in one sentence"),
                ChatMessage::user(text),
            ]);
            let client = Client::default();
            // Skip if does not have the environment name set
            if !OPENAI_ENV_NAME.is_empty() && std::env::var(OPENAI_ENV_NAME).is_err() {
                println!(
                    "===== Skipping model: {MODEL_OPENAI} (env var not set: {OPENAI_ENV_NAME})"
                );
            }
            let chat_res = client
                .exec_chat(MODEL_OPENAI, chat_req.clone(), None)
                .await?;
            let res = chat_res.content_text_into_string().unwrap_or_default();
            let client = reqwest::Client::new();
            let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);

            let res_mp3 = client
                .post(&url)
                .header("xi-api-key", eleven_labs_key.clone())
                .header("Content-Type", "application/json")
                .json(&json!({
                    "text": res,
                    "output_format": "mp3_44100_64",
                }))
                .send()
                .await?;

            let audio_data = res_mp3.bytes().await?;
            let audio_file = InputFile::memory(audio_data);

            bot.send_audio(msg.chat.id, audio_file).await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Send me plain text.").await?;
        }
    }
    Ok(())
}
