use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::env;
use dotenv::dotenv;

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
    candles: Vec<Candle>, 
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
    
    let candles_str = payload.candles.iter()
        .map(|c| format!("({}, {}, {}, {}, {})", c.time, c.open, c.high, c.low, c.close))
        .collect::<Vec<String>>()
        .join("\n");

    let prompt_text = format!(
        "あなたはプロのFXトレーダーです。以下の市場データに基づいて現状を分析してください。\n\
        対象通貨: {}\n\
        データ形式: 最新の足から過去30本分 (Time, Open, High, Low, Close)\n\n\
        【データ】\n{}\n\n\
        【評価軸】\n\
        1. トレンド方向 (上昇/下降/レンジ) とその強さ\n\
        2. 直近の注目すべきプライスアクション\n\
        3. 短期的な売買バイアス（強気/弱気/中立）\n\
        簡潔に箇条書きで出力してください。",
        payload.symbol, candles_str
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
