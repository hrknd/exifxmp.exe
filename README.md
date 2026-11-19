# Exif / XMP Viewer

PNGまたはJPEG画像をドラッグ＆ドロップし、埋め込まれたExifとXMPを表示するWindows向けデスクトップアプリです。

## 起動

事前に Visual Studio Installer から「C++によるデスクトップ開発」
（MSVCとWindows 10/11 SDK）をインストールしてください。

```powershell
cargo run --release
```

## 配布用ビルド

```powershell
cargo build --release
```

実行ファイルは `target/release/exifxmp.exe` に生成されます。

日本語表示にはWindows標準のメイリオを実行ファイルへ埋め込んで使用するため、
Exif値やXMP内の日本語も文字化けせず表示できます。


