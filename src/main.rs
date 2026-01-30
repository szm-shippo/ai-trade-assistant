use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::env;
use dotenv::dotenv;
use std::fs;

const PORT: u16 = 3000;

#[derive(Deserialize, Debug, Serialize)]
struct Candle {
    time: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Deserialize, Debug)]
struct Mt4Data {
    symbol: String,
    period: i32,
    candles: Vec<Candle>,
    low_period: i32,
    low_candles: Vec<Candle>, 
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let app = Router::new().route("/analyze", post(handle_analyze));

    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    println!("🚀 Rust Server started on port {}. Waiting for MT4 data...", PORT);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_analyze(Json(payload): Json<Mt4Data>) -> Json<Value> {
    println!("\n📈 Received data for: {}", payload.symbol);

    let format_candles = |candles: &Vec<Candle>| -> String {
        candles.iter()
            .map(|c| format!("({}, {:.3}, {:.3}, {:.3}, {:.3})", c.time, c.open, c.high, c.low, c.close))
            .collect::<Vec<String>>()
            .join("\n")
        };

    let base_candles_str = format_candles(&payload.candles);
    let low_candles_str = format_candles(&payload.low_candles);
    let strategy_instruction = fs::read_to_string("strategy.txt").unwrap_or_else(|_| {
        println!("Warning: strategy.txt not found! Using default instruction.");
        "あなたはFXトレーダーです。データを分析してください。".to_string()
    });

    let prompt_text = format!(
        "対象通貨: {}\n

        【上位足データ ({}分足)】 - トレンド把握用\n
        (Time, Open, High, Low, Close)\n
        {}\n\n

        【下位足データ ({}分足)】 - エントリータイミング用\n
        (Time, Open, High, Low, Close)\n
        {}\n\n

        {}",
        payload.symbol, payload.period, base_candles_str, payload.low_period, low_candles_str, strategy_instruction
    );

    match call_gemini_api(&prompt_text).await {
        Ok(analysis) => {
            println!("--------------------------------------------------");
            println!("{}", analysis);
            println!("--------------------------------------------------");
            Json(serde_json::json!({ "status": "success", "message": "Analysis printed to console" }))
        }
        Err(e) => {
            eprintln!("Error calling Gemini: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

async fn call_gemini_api(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let api_key = env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let model_name = env::var("MODEL_NAME").unwrap_or("gemini-3-flash-preview".to_string());

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    );

    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: prompt.to_string(),
            }],
        }],
    };

    let res = client.post(&url)
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        return Err(format!("API Error: {}", err_text).into());
    }

    let res_json: Value = res.json().await?;
    
    let text = res_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("No content generated")
        .to_string();

    Ok(text)
}
