---
version: alpha
name: Sumi / todo-list
description: 自分が手を動かすToDoを一覧し、リンクを開いて完了にするための画面契約。
colors:
  primary: "#2f6f4e"
  accent: "#2f6f4e"
  accent-subtle: "rgba(47, 111, 78, 0.10)"
  surface: "#faf6ef"
  surface-raised: "#fffdf8"
  on-surface: "#3a2f28"
  muted: "#6f6257"
  border: "#e3d9c9"
  scrim: "rgba(58, 47, 40, 0.4)"
  link: "#14506e"
  danger: "#9c2b1d"
  danger-subtle: "#f9e9e4"
  wash-base: "#f6efe0"
  hover-1: "rgba(47, 111, 78, 0.10)"
  hover-2: "rgba(47, 111, 78, 0.16)"
typography:
  title:
    fontFamily: system-ui
    fontSize: 17px
    fontWeight: 600
    lineHeight: 1.3
  body:
    fontFamily: system-ui
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.6
  body-sm:
    fontFamily: system-ui
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: system-ui
    fontSize: 15px
    fontWeight: 500
    lineHeight: 1.2
  caption:
    fontFamily: system-ui
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
rounded:
  sm: 6px
  md: 8px
  lg: 12px
  full: 9999px
spacing:
  sp-1: 4px
  sp-2: 8px
  sp-3: 12px
  sp-4: 16px
  sp-5: 24px
components:
  app-header:
    backgroundColor: "{colors.wash-base}"
    textColor: "{colors.on-surface}"
    height: 48px
  hairline:
    backgroundColor: "{colors.border}"
    height: 1px
  todo-row:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.md}"
    padding: 10px
  todo-link:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.link}"
    typography: "{typography.label}"
  todo-meta:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.muted}"
    typography: "{typography.caption}"
  todo-overdue:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.danger}"
    typography: "{typography.caption}"
  checkbox:
    backgroundColor: "{colors.accent}"
    size: 20px
  button:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: 8px
  button-hover:
    backgroundColor: "{colors.hover-1}"
  button-pressed:
    backgroundColor: "{colors.hover-2}"
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.surface-raised}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: 8px
  icon-button:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    size: 36px
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: 8px
  modal:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.lg}"
    padding: 16px
  modal-scrim:
    backgroundColor: "{colors.scrim}"
  radio-selected:
    backgroundColor: "{colors.accent-subtle}"
    rounded: "{rounded.sm}"
  error-banner:
    backgroundColor: "{colors.danger-subtle}"
    textColor: "{colors.danger}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 8px
  spinner:
    textColor: "{colors.accent}"
    size: 18px
---

# todo-list

## Overview

各アプリやAIエージェントが登録した「自分が手を動かすこと」を1つの一覧で見る道具である。
主な利用はスマートフォンで一覧を見て、項目のリンクを開いて作業し、戻って完了にすること。
登録は主にエージェントが行うため、画面からの追加は短い入力で済む補助操作とする。

Sumi familyのデザイン原本（rust-svelte-template、2026-09-26時点）を取り込み、この製品向けに編集した。
採用後は本書が正であり、原本の更新は差分を確認して明示的に取り込む。

## Colors

暗色はSumi、明色はKinariとする。frontmatterはKinariの値を持つ。
色は `client/src/global.sass` のCSS変数を使い、画面内に色を直書きしない。

| 役割 | Kinari | Sumi |
|---|---|---|
| surface | #faf6ef | #191919 |
| surface-raised | #fffdf8 | #232323 |
| on-surface | #3a2f28 | #e6e6e6 |
| muted | #6f6257 | #9a9a9a |
| border | #e3d9c9 | #333333 |
| accent / primary | #2f6f4e | #5fbf8a |
| accent-subtle | rgba(47,111,78,.10) | rgba(95,191,138,.15) |
| link | #14506e | #7fdbff |
| danger | #9c2b1d | #ff6b6b |
| danger-subtle | #f9e9e4 | #3a1a1a |
| scrim | rgba(58,47,40,.4) | rgba(0,0,0,.6) |
| wash-base | #f6efe0 | #232323 |
| hover-1 | rgba(47,111,78,.10) | #333333 |
| hover-2 | rgba(47,111,78,.16) | #3d3d3d |

アクセントは「済ませる」ことを表す緑とし、主操作の追加ボタン、完了チェック、フォーカスだけに使う。
塗りつぶす主操作は追加ボタンの1つだけとする。主ボタンの文字色はsurface-raisedで、
Kinari 5.9:1、Sumi 7.0:1のコントラストを持つ。
リンクのある項目名はlink色で示し、アクセントと混同させない。
期限切れはdanger色に加えて「期限切れ」の文字で示す。完了済みの項目名はmutedと取り消し線で示す。
通常文字は両テーマでWCAG AAの4.5:1以上を保つ。

### テーマの状態

`:root` をSumiの値と `color-scheme: dark` にする。
`data-theme="light"` でKinari、`data-theme="dark"` でSumiを明示指定する。
自動では属性と保存キー `todo-list:theme` を削除し、OSの `prefers-color-scheme` に従う。
Kinariの明示指定とOS委任は同じSass mixinから出力する。
設定はlocalStorageへ保存し、`index.html` の小さなscriptで初回描画前に適用する。

## Typography

書体はsystem-uiとし、Webフォントを追加しない。サイズ・太さ・行高はfrontmatterの5役割を使う。
項目名はlabel（15px・500）で折り返して全文を見せる。期日と登録元は注記（12px・muted）、
メモと失敗の説明は補助文（14px）とする。入力欄は本文サイズ（16px）でスマートフォンの拡大を防ぐ。

## Layout

### 画面構成

画面はホーム1枚だけで、URLは `/` とする。上から次の順に置く。

1. app header: 全幅・高さ48px・sticky。wash-baseと1pxの下境界を使う。
   左にホームへ戻るアプリ名、右に36pxのメニューボタンだけを置く。
2. 追加フォーム: ラベル「新しい ToDo」、入力欄、追加ボタンの1行。
   その下の折りたたみ「リンク・期日・メモ」に任意の入力を収め、既定は閉じる。
3. 未完了の一覧: 期日の近い順（期日なしは最後）、同じ期日は登録の古い順。
4. 「完了済み N件」の折りたたみ: 完了の新しい順。既定は閉じる。

件数の帯や説明文は置かない。未完了の件数は一覧そのもので分かる。
本文は通常の文書フローで、ページの縦スクロールを使う。mainに独立したスクロール領域を作らない。
本文の列は中央配置で最大720px、左右の余白12px、上下は狭幅16px・768px以上で24pxとする。
幅320px以上でページを横にはみ出させない。長い項目名は折り返す。

### 一覧の行

各行はsurface-raised、1pxの枠、8pxの角丸を持ち、行の間隔は8pxとする。
左に44px四方の完了チェック領域（20pxのネイティブcheckbox、accent-color）を置く。
右の本文は上下左右10px（左はチェック領域に接する）とし、次を縦に並べる。

- 項目名。リンクがあればlink色の行全幅のリンクで、末尾に外部リンクのアイコンを付ける。
  新しいタブで開く（`target="_blank" rel="noopener noreferrer"`）。http(s)以外はリンクにしない。
- 注記の行: 期日と「登録: 登録元」。期日の表記は「今日まで」「明日まで」「M/Dまで」、
  年が違う場合は「Y/M/Dまで」、過ぎたら「期限切れ M/D」とする。
  どちらも無ければ行ごと出さない。
- メモがあれば折りたたみ「メモ」。開くと改行を保った平文で見せる。Markdownとして解釈しない。

## Components

### 完了と取り消し

チェックを付けると即座にAPIへ保存し、行はその場で完了表示（チェック済み・muted・取り消し線）になる。
次の読み込みまで位置を変えないので、誤操作は同じチェックを外すだけで戻せる。
完了済みの折りたたみ内でチェックを外すと未完了に戻り、次の読み込みで一覧へ戻る。
保存に失敗したらチェックを元に戻し、その行の中に失敗の説明（error-banner、role=alert）を出す。
アクセシブルな名前は「完了: 項目名」とする。

### 追加フォーム

項目名だけで追加できる。Enterまたは追加ボタンで送る。送信中は追加ボタンを無効にする。
リンクは `type="url"` と http(s) の入力パターンで、ブラウザが送信前に誤りを示す。
期日は `type="date"`、メモは3行のtextareaとする。空の任意項目は送らない。
成功したら入力を空にして一覧を読み直す。失敗したら入力を残し、フォームの下に理由を示す。
ラベルは入力欄の上にmutedの注記で置く。

### 状態と再取得

一覧の `data-state` はloading/empty/error/successとする。
読込中は14pxの文字とスピナー、未完了が無ければ「やることはありません」、
取得失敗はdanger色の説明と「再試行」ボタンを示す。完了済みの折りたたみは空・成功のどちらでも出す。
タブが再表示されたら一覧を取り直す。リンク先から戻ったときにエージェントの追加が反映される。

### メニューとテーマ設定

メニューは右上のボタンに接するドロップダウンとする。上端をheader下端、右端をボタン右端へそろえる。
最小幅180px、1pxの枠と12pxの角丸、背景はsurface-raisedとする。項目は「テーマ設定」だけである。
背面の透明な閉じるボタンとEscで閉じ、フォーカスをメニューボタンへ戻す。開閉は `aria-expanded` に反映する。

テーマ設定は中央のモーダルで開く。自動・ライト・ダークの3つのラジオにmonitor/sun/moonのアイコンを使う。
選択は即時反映し、モーダルを閉じない。閉じるボタン・Esc・scrimで閉じ、フォーカスをメニューボタンへ戻す。

### 部品

- アイコンは `client/src/lib/Icon.svelte` の辞書だけを使う。24×24、currentColorの2px線、1.2em。
  絵文字や文字記号をアイコンの代わりにしない。列挙は `ICON_NAMES` を使う。
- 通常ボタンはsurface-raised、1px枠、6px角丸、上下8px・左右14pxの余白。ホバーはhover-1、
  無効時は不透明度50%。主ボタンはaccentで塗る。
- 入力欄はsurface、1px枠、6px角丸、本文サイズ。フォーカスはaccent色の枠と共通リング。
- モーダルは中央配置、12px角丸、16px余白とscrim。内容は最大80dvhで内部スクロールする。
- 影はメニューとモーダルの `0 8px 32px rgba(0,0,0,.25)` だけに使う。
- フォーカスは共通の `:focus-visible` に2pxのaccent色の輪郭と2pxの間隔を設ける。
- 動きは150ms以下の変化とスピナーに限り、`prefers-reduced-motion: reduce` では止める。

## Verification

実装はSassの字下げ構文を使い、normalize.cssを先に読み込む。
変数の接頭辞は色が `--c-*`、余白が `--sp-1..5`、文字が `--fs-xs..xl`、角丸が `--radius-*` とする。
`designmd lint` は形式を検査する。UIへの適用は `npm --prefix client run test:e2e` が
実binaryと一時データでChromiumを使って測る。

- 375×812・320×640・1280×720の明暗を測る。ページの横はみ出しは0pxとする。
  背景はSumiがrgb(25,25,25)、Kinariがrgb(250,246,239)で、期限切れはdanger色になる。
- 14件の架空データを使う。一覧の先頭行は画面上端から200px以内に置く。
  header下の全体が見える行は、812px高で7行以上、640px高で4行以上とする。
- スマートフォン幅の操作を、APIの保存結果と合わせて確認する。
  リンク・期日付きで追加し、リンクを新しいタブで開き、完了にする。
  再読み込み後は完了済みへ移り、チェックを外すと未完了へ戻る。
- キーボードで入力欄から追加ボタンへ移動でき、フォーカスリングが見える。Enterで追加できる。
- http(s)以外のリンクは送信前に止まり、入力が残る。読込中・空・取得失敗と再試行、
  完了の保存失敗でチェックが戻り行内に説明が出ることを確認する。
