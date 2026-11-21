#!/usr/bin/env bash
# Создаёт milestone, метки и issues по файлам .github/issues/*.md.
# Требуется GitHub CLI с авторизацией: brew install gh && gh auth login
#
#   ./scripts/create-issues.sh
#
# Повторный запуск безопасен: задачи с тем же заголовком пропускаются.

set -euo pipefail
cd "$(dirname "$0")/.."

REPO="${REPO:-eop10/unisono}"
MILESTONE="0.1.0 — MVP"
DUE="2027-01-31T00:00:00Z"

if ! command -v gh >/dev/null 2>&1; then
    echo "Ошибка: GitHub CLI не установлен. Установите: brew install gh" >&2
    exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
    echo "Ошибка: нет авторизации. Выполните: gh auth login" >&2
    exit 1
fi

echo "→ метки stage-0…stage-10"
for n in $(seq 0 10); do
    gh label create "stage-$n" --repo "$REPO" --color "1D76DB" --force >/dev/null 2>&1 || true
done

echo "→ milestone «${MILESTONE}»"
if ! gh api "repos/$REPO/milestones?state=all&per_page=100" --jq '.[].title' | grep -Fxq "$MILESTONE"; then
    desc="$(cat .github/milestone.md)"
    gh api -X POST "repos/$REPO/milestones" \
        -f title="$MILESTONE" \
        -f description="$desc" \
        -f due_on="$DUE" >/dev/null
    echo "  создан"
else
    echo "  уже есть"
fi

echo "→ issues"
for f in .github/issues/*.md; do
    base="$(basename "$f" .md)"
    case "$base" in
        README) continue ;;
    esac

    num="${base%%-*}"
    num=$((10#$num))
    title="$(head -n 1 "$f" | sed 's/^# //')"
    [ -z "$title" ] && continue

    if gh issue list --repo "$REPO" --state all --search "\"$title\" in:title" \
        --json title --jq '.[].title' | grep -Fxq "$title"; then
        echo "  пропуск: $title"
        continue
    fi

    gh issue create --repo "$REPO" \
        --title "$title" \
        --body "$(tail -n +2 "$f")" \
        --label "stage-$num" \
        --milestone "$MILESTONE" >/dev/null
    echo "  создан: $title"
done

echo "Готово: https://github.com/$REPO/issues"
