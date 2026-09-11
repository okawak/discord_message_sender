# 非圧縮WASMの配布サイズ調査

2026-09-10。Rust中心の構成、既存機能、標準ライブラリによる保守性を維持する前提で測定。

追記: 現在はRust **1.98.1**へ更新し、`core.rs`への改名と統合テストの`tests/`への移動後、独自ラッパーを廃止した標準Cargoビルドで、非圧縮の`main.js`は**907,780 bytes**、WASMは**890,363 bytes**。配布時は`cdylib`のみを生成し、LTOを維持する。以下の比較表はRust 1.98.0で測定した当時の記録として保持する。

## 採用した変更と結果

`dist/main.js`は **929,241 → 908,777 bytes**（20,464 bytes、約2.2%削減）。gzip/Brotliなどの圧縮は使用していない。外部WASMへの分離も行っていない。

- HTMLは既にUTF-8の`&str`なので、`html5ever`へ`TendrilSink::one(html)`で渡す。バイト列化、UTF-8デコーダー、`Read`経由の入力を省く。
- 日時は使用する5種類の固定形式をRustの整数書式で出力する。RFC 3339の解析、暦・ISO週・日付演算には引き続き`chrono`を使用する。負の年・5桁以上の年・うるう秒も従来のChrono書式に合わせる。

いずれもパーサーの独自実装ではない。`html5ever`、`chrono`、`regex-lite`、`thiserror`は維持した。

| 最終成果物の内訳 | bytes |
| --- | ---: |
| 元のWASMバイナリ | 662,304 |
| Base64文字列 | 883,072 |
| TS・生成JS・初期化コード等 | 25,705 |
| `dist/main.js`全体 | **908,777** |

Base64は3 bytesを4文字で表すため、このWASMでは220,768 bytes増える。TSを整理するだけで大幅には減らせない。1 MBの判定は1,000,000 bytesで行う。Viteが参考表示するgzipサイズは判定に使用しない。

## 独立した比較ビルド

以下の各実験は、特記がなければ変更前929,241 bytesを共通の基準にして一時コピーで実施した。表の削減量は単純加算できない。公開関数の削除など、採用しなかった変更は作業ツリーに反映していない。

| 変更 | WASM bytes | main.js bytes | 判断 |
| --- | ---: | ---: | --- |
| 基準: `opt-level=z`、LTO、`wasm-opt -Oz` | 677,652 | 929,241 | 比較元 |
| HTMLの文字列を直接入力 | 673,105 | 923,181 | 採用 |
| Chronoの`format_with_items`で書式解析を省略 | 672,381 | 922,213 | 固定形式出力の方が小さいため不採用 |
| Chronoの日時成分を固定形式で出力 | 666,838 | 914,825 | 採用 |
| TS本番コードが直接使わないWASM exportを11個削除 | 674,007 | 924,381 | 約4.9 kBのため、テスト用ビルドを分ける複雑さを避ける |
| Rust `opt-level=s` | — | 1,009,045 | 上限超過。先行測定 |
| Rust `opt-level=2` | 854,683 | 1,165,285 | 上限超過 |
| Rust `opt-level=3` | 881,007 | 1,200,381 | 上限超過 |
| 採用した2点の組み合わせ・整形後 | **662,304** | **908,777** | 最終状態 |

`wasm-opt 117`による追加最適化も基準WASMに対して測定した。`--converge`を付けた`-O1/-O2/-O3/-O4`は、それぞれ678,772 / 678,777 / 680,781 / 680,994 bytesに増加。`-Os`は677,347、`-Oz`は677,346 bytesとなり、最大でも306 bytesの削減だった。追加パスは採用していない。

最適化レベルとサイズの関係は単調ではなく、[Cargo公式も実測を推奨](https://doc.rust-lang.org/cargo/reference/profiles.html#opt-level)している。今回のビルドでは既存の`z`が有利だった。

さらに[Binaryen公式のversion_132](https://github.com/WebAssembly/binaryen/releases/tag/version_132)も、一時取得したmacOS arm64バイナリの公開SHA-256を照合して測定した。この比較だけは**採用後のコード**を使い、同じwasm-bindgen出力にそれぞれの最適化を適用した。

| 最適化ツール | 最終WASM bytes | 117との差 |
| --- | ---: | ---: |
| wasm-opt 117 `-Oz` | 662,304 | 基準 |
| wasm-opt 132 `-Oz` | 661,562 | −742 |
| wasm-opt 132 `-Oz --converge` | 661,122 | −1,182 |

最大でもBase64部分の削減は1,576 bytes。今回のサイズ対策としては、wasm-packが使用するツールの取得・固定方法を変えるほどの効果ではないため採用しなかった。132の出力についてはサイズ計測のみで、配布バンドルの互換性テストは行っていない。

## 大きな部分の特定

`wasm-bindgen --keep-debug`と`wasm-opt -Oz -g`で調査用の関数名を保持し、WASMのcode sectionを解析した。調査用name sectionは配布物には含めない。基準ビルドと調査用ビルドのcode/data sectionのサイズは一致した。

基準WASMはcode sectionが365,757 bytes、data sectionが303,921 bytes。大きな関数には以下があった。これはインライン化後の個々の関数本体のサイズで、ライブラリ全体の寄与率ではない。

| 関数 | 本体 bytes |
| --- | ---: |
| html5everのTreeBuilder::step | 28,825 |
| html5everのTokenizer::run | 14,360 |
| f64のDisplay | 9,567 |
| HTML変換の公開関数 | 9,000 |
| regex-liteのParser::parse_inner | 7,846 |
| ChronoのStrftimeItemsによるToString | 7,550 |
| dlmallocのmalloc | 4,738 |

HTML文字参照用の生成データには、名前の途中の接頭辞を含めて9,854エントリーある。wasm32上で各エントリーを16 bytesとして計算すると、エントリー配列だけで157,664 bytesになり、ほかに文字列や検索用データが必要になる。ただし文字列の共有・最適化があるため、生成ソースの長さをそのまま最終バイナリの寄与量とは扱わない。

**HTML変換のexportだけを外す診断ビルド**では、WASMが677,652 → 271,391 bytesになった。406,261 bytesの差は、HTMLパーサー、Markdownレンダラー、文字参照、その他の到達不能になった共通処理を含む。`html5ever`単体のサイズではない。この診断ビルドはクリッピング機能が欠落し、TSのビルドも未解決exportで失敗するため、配布可能な成果物ではない。

この結果から、大幅な削減を検討するならHTML処理が優先対象と判断した。WASM形式そのものに約900 kBという下限があるわけではない。

## ライブラリ・構成の候補

以下はドキュメントと現在のソースを調査した候補で、置換後のサイズ・互換性は未測定。

| 候補 | 調査結果・今回の判断 |
| --- | --- |
| `html5ever`のfeatures削減 | 現在の0.39.0は`serde`と`trace_tokenizer`が任意機能。文字参照やHTML構文回復を個別に切るfeatureはない。`default-features=false`だけで大きくは減らせない |
| `tl` | RustのDOMパーサーだが、HTML規格全体には従わず、不正なタグの扱いで内容を落とす場合があると公式に明記。現在のクリッピング結果を保つ置換とは見なせない。[公式](https://docs.rs/tl/0.7.8/tl/) |
| `html5gum` | HTML tokenizer。単体ではDOM構築や誤った入れ子の修復を行わない。`tree-builder` featureでhtml5everと組み合わせる方法はあり、将来の比較候補。ただし今回の最終サイズは未測定。[公式](https://docs.rs/html5gum/0.8.4/html5gum/) |
| `lol_html` | ストリーミングでHTMLを書き換えるAPIが中心。現在のDOMを走査するMarkdown変換への単純な差し替えではない。[公式](https://docs.rs/lol_html/latest/lol_html/) |
| ホストの`DOMParser`をRustから呼ぶ | HTML解析をホストに任せ、Markdown変換をRustに残す構成は可能。HTMLパーサーの同梱を省ける候補だが、DOM→内部構造の橋渡し、HTML出力互換性、Obsidianのデスクトップ・モバイル実環境の検証が必要。実際の削減量は未測定。[HTML標準](https://html.spec.whatwg.org/multipage/dynamic-markup-insertion.html#dom-domparser-parsefromstring) |
| アロケーター差し替え | プロファイルでdlmallocのコードは目立つが、主要なHTMLデータは減らない。安全性・長時間利用時のメモリ特性の検証に見合う効果を未確認のため未実装 |
| `serde`/`tsify`削減、JSON文字列だけの境界 | 型生成や変換エラーの扱いを保つため今回は維持。JS/WASM両側のJSON処理を増やすため、単純に小さくなるとは限らない |
| `thiserror`削除 | 手書きのError実装に相当するコードを生成するライブラリ。大幅なサイズ削減につながらないため維持。[公式](https://docs.rs/thiserror/latest/thiserror/) |
| 正規表現の独自実装 | ユーザーの方針に従い再実装しない。`regex-lite`を維持 |
| 外部`.wasm`、別の文字列符号化、圧縮 | 今回の構成・目的の対象外。配布物全体を軽くしたこととmain.jsからバイト列を移しただけのことを混同しない |

## 検証と再測定

測定環境: macOS arm64、Rust 1.98.0、Bun 1.4.0、wasm-pack 0.15.0、wasm-bindgen 0.2.128、wasm-opt 117。依存はコミット対象のCargo.lockとbun.lockに従う。

```sh
bun run build
wc -c dist/main.js pkg/parse_message_bg.wasm
bun run type-check
bun run check
bun run test
bun run test:wasm
bun run verify:build
```

採用後にRust324件・Bun103件、型検査、Biome、ネイティブとWASMのclippy、実配布バンドルのオフライン初期化テストを通過。日時の追加テストは負の年・年0・5桁以上の年、各月末、うるう秒について以前のChrono書式と比較する。HTML変換は既存294件の単体テストと2件の統合テストで確認した。実Obsidian UIの操作テストは今回行っていない。

独立した一時ディレクトリからのビルドでmain.jsとmanifest.jsonのSHA-256が一致した。最終main.js: `1946d89374a00a113ffcccbcc8d382189d4bf28f4781f556793bc1d0d38aab39`。

計測は[wasm-bindgen公式](https://wasm-bindgen.github.io/wasm-bindgen/reference/optimize-size.html)に従い、Rustコンパイラーが直接出力する未処理WASMではなく、wasm-bindgenとwasm-optを通した成果物を対象とする。
