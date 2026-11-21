# Конфигурация сборки документации.
# Полный список параметров: https://www.sphinx-doc.org/en/master/usage/configuration.html

# -- Сведения о проекте ------------------------------------------------------

project = 'Unisono'
copyright = '2026, Команда Unisono'
author = 'Системный аналитик'

# Версия документа: показывается в подвале и в метаданных HTML.
release = '0.2'
version = '0.2'

# -- Общие настройки ---------------------------------------------------------

extensions = [
    'myst_parser',                    # Markdown как полноценный источник
    'sphinx.ext.autosectionlabel',    # ссылки на заголовки по :ref:
    'sphinx.ext.mathjax',             # формулы из MyST dollarmath
    'sphinx.ext.intersphinx',         # внешние ссылки (python, django и т. п.)
]

# Документы пишутся в Markdown; .rst оставлен для index.rst.
source_suffix = {
    '.md': 'markdown',
    '.rst': 'restructuredtext',
}

language = 'ru'
root_doc = 'index'
templates_path = ['_templates']
exclude_patterns = [
    '_build',
    '**/_build',
    '_legacy',              # исходные версии документов, не входят в сборку
    'diagrams/src',         # исходники .mmd, рендерятся в diagrams/*.png
    'Thumbs.db',
    '.DS_Store',
    'README.md',
]

# -- MyST (Markdown) ---------------------------------------------------------

myst_enable_extensions = [
    'deflist',        # списки определений
    'colon_fence',    # :::note / :::warning
    'tasklist',       # чекбоксы
    'attrs_inline',   # {.class} и #id прямо в тексте
    'dollarmath',     # $...$ и $$...$$ — формулы MathJax
    'amsmath',        # выровненные окружения \begin{align}
    'linkify',        # автоссылки на URL
]

myst_heading_anchors = 3          # якоря до h4 — нужны для внешних ссылок
myst_links_external_new_tab = True

# -- Ссылки на заголовки -----------------------------------------------------

# Иначе одинаковые подзаголовки («Ошибки», «Поля») в разных файлах конфликтуют.
autosectionlabel_prefix_document = True
autosectionlabel_maxdepth = 3

suppress_warnings = [
    'myst.header',
]

# -- HTML: alabaster ---------------------------------------------------------

html_theme = 'alabaster'

html_theme_options = {
    'description': 'Unisono — мультиинструментальный тюнер на Rust + Tauri для macOS, Windows, Linux и Android',
    # Ширина: таблицы и JSON-примеры требуют больше места, чем дефолтные 940 px.
    'page_width': '1400px',
    'sidebar_width': '280px',
    'body_max_width': 'none',
    'fixed_sidebar': True,
    'show_relbars': True,
    'github_button': False,
    'nosidebar': False,
    # Системный шрифт без загрузки веб-шрифтов: документ должен одинаково
    # выглядеть офлайн и на любой машине.
    'font_family': "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif",
    'head_font_family': "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif",
    'font_size': '15px',
    'code_font_size': '0.82em',
}

html_title = 'Unisono — технический проект'
html_short_title = 'Unisono'
html_static_path = ['_static']
html_css_files = ['custom.css']
html_last_updated_fmt = '%d.%m.%Y'

# -- Прочее ------------------------------------------------------------------

highlight_language = 'kotlin'
pygments_style = 'friendly'
smartquotes = True

# MathJax лежит в _static: формулы отрисовываются без интернета (важно для PDF и офлайна).
mathjax_path = 'mathjax/tex-mml-chtml.js'

intersphinx_mapping = {
    'python': ('https://docs.python.org/3', None),
}
