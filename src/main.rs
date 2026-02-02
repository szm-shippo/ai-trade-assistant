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
use chrono::Local;
const PORT: u16 = 3000;

#[derive(Deserialize, Debug, Serialize)]
struct Candle {
    time: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mt4Data {
    symbol: String,
    period: i32,
    candles: Vec<Candle>,
    mid_period: i32,
    mid_candles: Vec<Candle>,
    low_period: i32,
    low_candles: Vec<Candle>, 

    #[serde(default)]
    sub_symbol: String,
    #[serde(default)]
    sub_symbol_period: i32,
    #[serde(default)]
    sub_candles: Vec<Candle>, 
    #[serde(default)]
    sub_symbol_low_period: i32,
    #[serde(default)]
    sub_low_candles: Vec<Candle>,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Content {
    role: Option<String>,
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Part {
    text: String,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Content,
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

    save_json_log(&payload);

    let format_candles = |candles: &Vec<Candle>| -> String {
        candles.iter()
            .map(|c| format!("({}, {:.3}, {:.3}, {:.3}, {:.3})", c.time, c.open, c.high, c.low, c.close))
            .collect::<Vec<String>>()
            .join("\n")
        };

    let base_candles_str = format_candles(&payload.candles);
    let mid_candles_str = format_candles(&payload.mid_candles);
    let low_candles_str = format_candles(&payload.low_candles);
    let sub_candles_str = format_candles(&payload.sub_candles);
    let sub_low_candles_str = format_candles(&payload.sub_low_candles);

    let strategy_instruction = fs::read_to_string("strategy.txt").unwrap_or_else(|_| {
        println!("Warning: strategy.txt not found! Using default instruction.");
        "あなたはFXトレーダーです。データを分析してください。".to_string()
    });

    let now = Local::now();
    let current_time_str = now.format("%Y年%m月%d日 %H:%M:%S").to_string();

    let prompt_text = format!(
        "=== メイン分析対象: {} ===

        現在時刻: {}

        【上位足 ({}分足)】 - 環境認識
        (Time, Open, High, Low, Close)
        {}

        【中位足 ({}分足)】 - 詳細分析用
        (Time, Open, High, Low, Close)
        {}

        【下位足 ({}分足)】 - エントリータイミング用
        {}

        === 相関確認対象: {} ===
        (メイン通貨ペアとの同調・乖離を確認してください)

        【相関・上位足 ({}分足)】
        {}

        【相関・下位足 ({}分足)】
        {}

        {}",
        payload.symbol, current_time_str, payload.period, base_candles_str, payload.mid_period, mid_candles_str, payload.low_period, low_candles_str, payload.sub_symbol, payload.sub_symbol_period, sub_candles_str, payload.sub_symbol_low_period, sub_low_candles_str, strategy_instruction
    );

    match call_gemini_api(&prompt_text).await {
        Ok(analysis) => {
            println!("--------------------------------------------------");
            println!("{}", analysis);
            println!("--------------------------------------------------");
            Json(serde_json::json!({
                "status": "success",
                "symbol": payload.symbol,
                "analysis": analysis
            }))
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
            role: Some("user".to_string()),
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

    let response: GeminiResponse = res.json().await?;
    
    let text = response.candidates
        .as_ref()                          // Optionの中身を借用
        .and_then(|c| c.first())           // candidates配列の先頭を取得
        .and_then(|c| c.content.parts.first()) // parts配列の先頭を取得
        .map(|p| p.text.clone())           // テキストをコピー
        .unwrap_or_else(|| "No analysis generated".to_string());

    Ok(text)
}

fn save_json_log(payload: &Mt4Data) {
    let log_dir = "logs/data/";

    if let Err(e) = fs::create_dir_all(log_dir) {
        eprintln!("Failed to create log directory: {}", e);
        return;
    }

    let now = Local::now();
    let filepath = format!("{}/log_{}_{}.json", 
        log_dir,
        payload.symbol, 
        now.format("%Y%m%d_%H%M%S")
    );

    match serde_json::to_string_pretty(payload) {
        Ok(json_content) => {
            if let Err(e) = fs::write(&filepath, json_content) {
                eprintln!("Failed to write log file: {}", e);
            } else {
                println!("Log saved: {}", filepath);
            }
        },
        Err(e) => {
            eprintln!("Failed to serialize payload: {}", e);
        }
    }
}

fn save_prompt_log(symbol: &str, prompt_content: &str) {
    let log_dir = "logs/prompts/";

    if let Err(e) = fs::create_dir_all(log_dir) {
        eprintln!("Failed to create log directory: {}", e);
        return;
    }

    let now = Local::now();
    let filepath = format!("{}/prompt_{}_{}.txt", 
        log_dir,
        symbol, 
        now.format("%Y%m%d_%H%M%S")
    );

    if let Err(e) = fs::write(&filepath, prompt_content) {
        eprintln!("Failed to write prompt log: {}", e);
    } else {
        println!("Prompt log saved: {}", filepath);
    }
}
