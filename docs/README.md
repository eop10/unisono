# Unisono — технический проект

Проект документации по мультиинструментальному тюнеру **Unisono** на **Rust + Tauri 2**
для **Android и macOS**: требования, архитектура, DSP, айдентика, план работ и ADR.

Сборка — Sphinx + тема `alabaster`, исходники разделов в Markdown (`myst-parser`).

## Сборка

```bash
python3 -m venv .venv
./.venv/bin/pip install -r requirements.txt
make html          # -> build/html/index.html
```

Сборка идёт с `-W --keep-going`: любое предупреждение (битый `{ref}`, дубль заголовка,
незакрытая метка) считается ошибкой.

## Показ и раздача

| Команда | Что делает |
| --- | --- |
| `make serve` | статический сервер, печатает адрес в локальной сети |
| `make pdf` | PDF через headless Chrome (LaTeX не нужен) → `dist/documentation.pdf` |
| `make bundle` | zip со статическим сайтом → `dist/` |
| `make share` | html + pdf + bundle |
| `make diagrams` | пересобрать PNG из `source/diagrams/src/*.mmd` |
| `make clean` | удалить `build/` и `dist/` |

`make pdf` и `make bundle` требуют Chrome:
`/Applications/Google Chrome.app/Contents/MacOS/Google Chrome` или переменную
`PUPPETEER_EXECUTABLE_PATH`.

## Структура

```text
source/
├── index.rst          титульная страница и toctree
├── conf.py            конфигурация Sphinx (alabaster, ru, myst, mathjax)
├── 01_intro.md        о документе
├── 02_requirements.md ФТ, НФТ, user stories, use cases
├── 03_architecture.md варианты стека, компоненты, поток данных
├── 04_dsp.md          NSDF/MPM, параметры окна, точность, тесты
├── 05_data.md         настройки, пресеты, миграции
├── 06_ui.md           экраны, состояния, доступность, тексты
├── 07_platform.md     Android, macOS, подпись, notarization
├── 08_build_test.md   Gradle, фикстуры, CI, чек-лист приёмки
├── 09_plan.md         этапы, оценки, диаграмма Ганта
├── 10_risks.md        риски, допущения, открытые вопросы ⚠️
├── 11_adr.md          ADR-001…ADR-005
├── 12_glossary.md     глоссарий
└── diagrams/
    ├── src/*.mmd      исходники Mermaid (источник истины)
    └── *.png          собранные рисунки (коммитятся)
tools/
├── render-diagrams.sh Mermaid → PNG
├── serve.sh           показ по локальной сети
├── build-pdf.sh       PDF из singlehtml через Chrome
└── bundle.sh          архив со статическим сайтом
```

## Правила оформления

- Нумерация разделов и рисунков ручная и сквозная; автонумерация не используется.
- Ссылки между разделами — `{ref}` с явными метками (`(sec-req)=` в начале файла).
- Рисунки — `figure` с подписью «Рис. N» и меткой `fig-...`.
- В таблицах символ `|` внутри ячейки экранируется как `\|`.
