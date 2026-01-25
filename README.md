# MT4 Gemini Market Analyzer (Rust CLI)

MetaTrader 4 (MT4) のチャートデータ（ローソク足）をリアルタイムで取得し、Rust製のCLIサーバーを経由して Google Gemini (Generative AI) に市場分析を行わせるツールです。

## 特徴
  - MT4連携: MQL4 EAを使用して、新しい足が確定するたび（またはチャート適用時）にデータを自動送信。
  - 高速・軽量: バックエンドサーバーは Rust (Axum) で記述されており、高速かつ低リソースで動作します。
  - AI分析: 最新の Gemini モデル（例: Gemini 2.0 Flash）を使用して、トレンド、プライスアクション、売買バイアスを言語化して出力します。

## システム構成

```graph LR
  A[MT4 Client (EA)] -- HTTP POST (JSON) --> B[Rust CLI Server (localhost:5000)]
  B -- REST API --> C[Google Gemini API]
  C -- Analysis Text --> B
  B --> D[Terminal Output]
```

## 必要要件

  - OS: Windows (MT4が動作する環境)
  - MetaTrader 4 (MT4): インストール済みであること
  - Rust: ツールチェーン (cargo) がインストールされていること
  - Google Gemini API Key: [Google AI Studio](https://aistudio.google.com/) から取得可能

## インストールとセットアップ

### 1. プロジェクトのクローンと依存関係のインストール

```bash
git clone https://github.com/szm-shippo/ai-trade-assistant.git
cd mt4_gemini_cli
cargo build
```

### 2. Rustサーバーの設定

`src/main.rs`を開き、GeminiのAPIキーを設定してください。
```rust
const GEMINI_API_KEY: &str = "APIキーを貼り付け"; 
const MODEL_NAME: &str = "gemini-2.0-flash";
```

### 3. MT4 (MQL4) の設定

  - MT4の「メタクォーツ言語エディタ (MetaEditor)」を開きます。

  - 新規エキスパートアドバイザ (EA) を作成し、MQL4/Experts フォルダに保存します（例: GeminiConnector.mq4）。

  - 以下のコードをコピー＆ペーストしてコンパイルします。(リポジトリ内の mql4/GeminiConnector.mq4 を参照)

  - MT4のメニューから 「ツール」 > 「オプション」 > 「エキスパートアドバイザ」 を開きます。

  - 「WebRequestを許可するURLリスト」 にチェックを入れ、以下を追加します。(http://localhost:5000)

## 使い方

### 1. Rustサーバーの起動

ターミナル（コマンドプロンプトやPowerShell）で以下のコマンドを実行し、サーバーを待機状態にします。

```bash
cargo run
```

### 2. MT4でEAを稼働

  - MT4のナビゲーターウィンドウから、作成した GeminiConnector を任意のチャート（例: USDJPY 1時間足）にドラッグ＆ドロップします。
  - 「自動売買」ボタンがONになっている必要はありません（スクリプト実行権限のみで動作します）。

### 3. 分析結果の確認

Rust側のターミナルに、以下のような分析結果がリアルタイムで出力されます。

```
📈 Received data for: USDJPY
--------------------------------------------------
* トレンド方向: 上昇トレンド（調整局面）
* プライスアクション: 直近で長い下ヒゲが出現しており、買い圧力を示唆
* 売買バイアス: やや強気
--------------------------------------------------
```

## カスタマイズ

  - 分析の観点を変える: `src/main.rs`内の`prompt_text`を編集することで、Geminiへの指示を変更できます（例：「スキャルピング目線で分析して」「日本語ではなく英語で出力して」など）。
  - 取得本数の変更: MQL4側の`int bars = 30;`を変更することで、Geminiに渡す過去データの期間を調整できます。

## 免責事項

このツールは教育および研究目的で作成されています。生成される分析結果はAIによる予測であり、金融アドバイスではありません。実際の取引による損失について、開発者は一切の責任を負いません。

## License

MIT License