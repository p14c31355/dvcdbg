---

### 📄 `CONTRIBUTING.md`

````md
# コントリビューションガイドライン

ようこそ！このリポジトリへの貢献を歓迎します 🙌  
以下のガイドラインに従って、バグ報告・機能追加・改善を行ってください。

---

## 🔧 開発環境のセットアップ

### 前提

- Rust (最新版推奨)
- `cargo` / `rustup` インストール済み
- 組込みターゲットの場合、ターゲット向け toolchain も必要（例：`avr-hal`, `esp-idf`, `thumbv7em`）

```sh
# Rust nightly が必要な場合
rustup install nightly
````

### 依存の取得

```sh
cargo check
```

---

## 🐛 バグ報告

[Bug Report テンプレート](.github/ISSUE_TEMPLATE/bug_report.md) に従って Issue を作成してください。
可能であれば **再現コード** や **I2C ログ** を添付してください。

---

## ✨ 機能提案

[Feature Request テンプレート](.github/ISSUE_TEMPLATE/feature_request.md) を使って提案を Issue に投稿してください。

* 既存ドライバとの互換性や制約に留意してください。
* `no_std` 対応は明記してください。
* コマンド仕様の変更やマクロ追加には rationale を添えてください。

---

## 🔃 プルリクエスト

1. **Issue を立ててからブランチを切る**
   ブランチ名例: `fix/init-error`, `feat/drawtarget-support`

2. **テストと `cargo check` に通ること**

3. **PR テンプレートに沿って説明を記述**

4. PR コメントで関連 Issue をクローズするように書いてください:

   ```text
   Closes #42
   ```

---

## 🧪 テストポリシー

* 基本は `cargo test` が通ること
* `no_std` ターゲットではコンパイルが通ること
* 実機テスト（I2C/SPI）は環境によって CI 対象外です（目視確認で OK）

---

## 📦 コーディング規約

* `rustfmt` 準拠
* `clippy` 警告なしを推奨
* `std` / `no_std` 両対応の場合、`#[cfg(feature = "std")]` を利用
* ログ出力には `ufmt` / `defmt` など軽量ロガーを使用（`std` 非依存）

---

## 🤝 ライセンス

このプロジェクトは MIT ライセンスです。
貢献されたコードも同様に MIT ライセンスの下で公開されます。

---

## 💬 コンタクト

* メンテナ: [p14c31355](https://github.com/p14c31355)
* Issue または Discussions にお気軽にどうぞ！

---