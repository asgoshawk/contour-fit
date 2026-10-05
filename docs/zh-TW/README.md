# contour-fit

把剪影 PNG 轉成 SVG 曲線，並用誤差報告檢查曲線與原始輪廓的差距。
適合單色標誌、圖示與遮罩。命令列工具在本機執行；Rust 程式庫則可直接處理記憶體中的遮罩。

目前版本為 `0.1.0-alpha.1`，尚未發布，請先從原始碼建置。
[English README](../../README.md)

## 看看實際效果

![鳥形剪影：左側是原始 PNG，右側是擬合後的 SVG，兩者皆以白色背景展示。](../assets/demo/bird-comparison.png)

這是第一張使用者提供的實際範例。SVG 保留原始 1254 × 1254 畫布與透明背景；
圖中的白色面板只用於展示。

| 項目 | 結果 | 意義 |
| --- | --- | --- |
| SVG 大小 | 3,292 bytes | 以曲線儲存形狀。 |
| 曲線段數 | 62 | 涵蓋擷取出的全部 3 個輪廓。 |
| 誤差上界 | 0.983 px | 通過設定的 1 px 容許誤差。 |
| 像素重疊率（IoU） | 99.49% | 在原始解析度下比較 PNG 與 SVG 的形狀。 |

數字只代表這張圖。距離參考是依指定門檻從 PNG 擷取的輪廓，並非未知的原始向量圖。

[原始 PNG](../assets/demo/bird-source.png) · [產生的 SVG](../assets/demo/bird.svg) ·
[完整報告](../assets/demo/bird-report.json) · [示範素材與授權說明](../assets/demo/README.md)

## 先試內附範例

安裝 [Rust／rustup](https://rustup.rs/) 後執行；repo 已固定使用 Rust 1.97.0。

```sh
git clone https://github.com/asgoshawk/contour-fit.git
cd contour-fit
cargo build --release --locked
mkdir -p artifacts
./target/release/contour-fit docs/assets/demo/bird-source.png \
  -o artifacts/bird.svg \
  --report artifacts/bird-report.json \
  --debug-dir artifacts/bird-debug
```

開啟 `artifacts/bird.svg` 看向量結果。JSON 報告記錄誤差與各階段耗時；
`bird-debug/preview.png` 是 SVG 的預覽，`bird-debug/overlay.png` 則以紅線標示
擷取輪廓、藍線標示擬合曲線。

再次執行時請換新的輸出名稱：工具不會覆寫既有檔案。上層目錄須先建立，
工具只會建立選用的診斷目錄。轉換過程不使用網路。

## 換成自己的圖

在 repo 根目錄執行，把 `input.png` 換成圖片路徑：

```sh
./target/release/contour-fit input.png -o artifacts/output.svg
```

預設會從含透明像素的圖片讀取 alpha；完全不透明的圖片則以較暗的像素作為前景。
白底黑色剪影可直接使用預設值，其他常見需求如下：

| 需求 | 加上的參數 |
| --- | --- |
| 明確選擇透明度 | `--channel alpha` |
| 不透明黑底上的白色剪影 | `--channel luminance --invert` |
| 將容許誤差縮小至四分之一像素 | `--tolerance 0.25` |
| 檢查擬合結果 | `--report artifacts/output-report.json --debug-dir artifacts/output-debug` |
| 略過擬合後的曲線合併 | `--no-merge` |

容許誤差越小，通常需要更多曲線與運算。所有參數可用
`./target/release/contour-fit --help` 查看。

## 品質與限制

預設容許誤差為**原始影像尺寸下的 1 像素**。擬合後會獨立檢查距離、
分離的元件與巢狀孔洞；SVG 座標四捨五入後，也會再次驗證才寫入檔案。

工具保留畫布與小細節，包括零散的點，不會自動去雜點、刪除孔洞或裁切。
SVG 使用 `currentColor` 與 `evenodd` 填色，嵌入網頁時可繼承文字顏色。

第一版只處理剪影 PNG，尚未涵蓋照片分割、動畫 PNG 或彩色向量化。
檢查採有限精度運算，不是形式化證明；雜訊很多或容許誤差很小的輸入可能
超出資源限制，此時會回報失敗，不產生 SVG。
[完整量測定義與數值限制](../quality.md)。

## 開發與進一步閱讀

功能 PR 合入 `develop`；發布流程為 `feature → develop → release/X.Y.Z → production`。
Alpha 候選版在 release 分支，正式版在 `production`，不會自動發布。
TDD、程式碼規範與本機檢查見 [CONTRIBUTING](../../CONTRIBUTING.md)。

- [英文 README 的程式庫範例](../../README.md#use-the-rust-library)：從記憶體中的遮罩開始。
- [架構摘要](architecture.md)：各模組如何合作。
- [套件審查](../dependencies.md)：選用理由、授權與供應鏈安全。
- [效能紀錄](../performance.md)：時間、記憶體與重現方式。
- [發布文件](../releases.md)：分支、版號與 GitHub Free 的額度限制。

程式碼採 [MIT 授權](../../LICENSE)。第三方套件與圖片素材保留各自授權；
示範鳥圖的說明見 [素材文件](../assets/demo/README.md)。
