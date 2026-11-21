# Minimal makefile for Sphinx documentation
#
# Проект документации: Sphinx + тема alabaster.
# Сборка:   make html      -> build/html/index.html
# Показ:    make serve      -> ссылка для команды в локальной сети
#           make pdf        -> dist/<проект>.pdf
#           make bundle     -> dist/<проект>-html-<дата>.zip
#           make share      -> всё три сразу
# Чистка:   make clean

SPHINXOPTS    ?= -W --keep-going
SPHINXBUILD   ?= ./.venv/bin/sphinx-build
SOURCEDIR     = source
BUILDDIR      = build

.PHONY: help Makefile live clean serve pdf bundle share diagrams

help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

live:
	@./.venv/bin/sphinx-autobuild "$(SOURCEDIR)" "$(BUILDDIR)/html" $(SPHINXOPTS)

# Показ команде по локальной сети: печатает адрес вида http://192.168.x.x:8010/
serve:
	@./tools/serve.sh

# PDF для менеджеров (A4 landscape) без LaTeX — через headless Chrome
pdf:
	@./tools/build-pdf.sh

# Архив со статическим сайтом для отправки в чат
bundle:
	@./tools/bundle.sh

# Всё, что нужно, чтобы показать: сайт, PDF и архив
share: html pdf bundle

# Диаграммы: Mermaid .mmd -> PNG (нужен Node.js и Chrome)
diagrams:
	@./tools/render-diagrams.sh

clean:
	@rm -rf "$(BUILDDIR)" dist

%: Makefile
	@$(SPHINXBUILD) -M $@ "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
