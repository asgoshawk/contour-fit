# contour-fit

將剪影 PNG 擬合成精簡 SVG 的 Rust 程式庫與命令列工具。
目前開發版本為 `0.1.0-alpha.1`，尚未發布。

[English README](../../README.md) · [架構](architecture.md)

支援透明／黑白 PNG、多個輪廓與巢狀孔洞。預設最大誤差為原始影像的
1 像素，先滿足誤差與拓撲檢查，再減少曲線段；不會自動刪除孔洞、去雜點或裁切。
照片分割、LLM、彩色向量化及圖形介面不在第一版範圍內。

```sh
cargo build --release --locked
./target/release/contour-fit input.png -o output.svg \
  --report metrics.json --debug-dir debug
```

輸出位置不得已有檔案。SVG 使用單一 `currentColor` 路徑與封閉子路徑，
保留原畫布；座標四捨五入後會重新驗證。診斷輸出包含 PNG 預覽、紅色參考輪廓／
藍色擬合輪廓疊圖，以及報告中的 IoU。

誤差參考是指定門檻擷取出的等值輪廓，並非未知的原始向量圖。
距離上界與拓撲檢查採有限精度運算，不是形式化證明；限制詳見英文品質文件。

分支流程為 `feature → develop → release/X.Y.Z → production`。
Alpha 使用 `0.1.0-alpha.N`，正式版使用 `0.1.0`；發布前須通過測試與安全檢查，
並取得維護者明確批准。採 MIT 授權，第三方套件與圖片素材保留各自授權。

CI 使用公開 repo 的標準 Linux 與 Apple Silicon runner，執行時間免費；
依賴稽核併入 Linux 工作。候選版本手動建置，產物保留 7 天，儲存空間仍須
留意 GitHub Free 的額度。詳細限制見 [英文發布文件](../releases.md#github-free-and-ci-scope)。
