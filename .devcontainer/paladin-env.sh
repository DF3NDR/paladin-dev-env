#!/usr/bin/env bash
# Import host-provided credentials into the devcontainer environment.
#
# Sourced (not executed) from ~/.bashrc by .devcontainer/post-start.sh, and safe
# to source repeatedly.
#
# Also reached via BASH_ENV (set in docker-compose.yml and .claude/settings.json).
# ~/.bashrc only sources this file AFTER Debian's `case $- in *i*)` early exit, so
# non-interactive shells -- `bash -c`, `bash -l`, git hooks, Makefile recipes, AI
# coding agents -- never got here and saw no credentials. BASH_ENV closes that gap:
# non-interactive bash sources whatever it names. Keep this file idempotent and
# side-effect-free for that reason.
#
# HOW IT WORKS
# ------------
# The host directory ~/.config/paladin is bind-mounted read-only at
# /home/vscode/.config/paladin (see .devcontainer/docker-compose.yml). Each file in
# it holds ONE secret, and the FILENAME IS THE VARIABLE NAME, lowercased:
#
#     ~/.config/paladin/gemini_api_key      ->  GEMINI_API_KEY
#     ~/.config/paladin/xai_api_key         ->  XAI_API_KEY
#     ~/.config/paladin/moonshot_api_key    ->  MOONSHOT_API_KEY   (Kimi)
#     ~/.config/paladin/dashscope_api_key   ->  DASHSCOPE_API_KEY  (Qwen)
#     ~/.config/paladin/zai_api_key         ->  ZAI_API_KEY        (Z.ai)
#     ~/.config/paladin/gh_token            ->  GH_TOKEN           (gh CLI)
#
# The mapping is generic — no hardcoded provider list — so a new provider needs a
# new file, not a change here.
#
# WHY THE HOST, NOT THE CONTAINER
# -------------------------------
# /home/vscode is part of the container filesystem and is DESTROYED on every
# rebuild. A key written there would not survive one. Keeping it on the host also
# means it is never in the image, never in the repo, and shared with any other
# project following the same convention.
#
# PRECEDENCE
# ----------
# A file is applied when the variable is currently unset OR empty. The repo .env is
# auto-sourced into every shell and declares these names with EMPTY values, so
# without the empty-check those blanks would mask real keys. A genuinely exported
# non-empty value always wins, so one-off overrides still work.
#
# shellcheck shell=bash

(return 0 2>/dev/null) || {
    echo "paladin-env.sh must be sourced, not executed: . .devcontainer/paladin-env.sh" >&2
    exit 64
}

PALADIN_SECRETS_DIR="${PALADIN_SECRETS_DIR:-$HOME/.config/paladin}"

__paladin_load_secrets() {
    [ -d "$PALADIN_SECRETS_DIR" ] || return 0
    local f name value current
    for f in "$PALADIN_SECRETS_DIR"/*; do
        [ -f "$f" ] && [ -r "$f" ] || continue
        name="$(basename "$f")"
        # Skip editor/backup noise and dotfiles; accept only plausible var names.
        case "$name" in
            .*|*~|*.bak|*.swp|*.example|*.md|README*) continue ;;
        esac
        [[ "$name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
        name="${name^^}"

        current="${!name:-}"
        [ -n "$current" ] && continue

        # Trim CR/LF and surrounding whitespace: a key saved by an editor or written
        # with `echo` carries a newline that would corrupt the auth header.
        value="$(tr -d '\r\n' < "$f" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
        case "$value" in
            '' | '<'*'>' | 'REPLACE_ME'* | 'your'* | '#'*) continue ;;
        esac
        export "$name=$value"
    done
}

__paladin_load_secrets

# `paladin-keys` — report which credentials resolved, WITHOUT printing any value.
paladin-keys() {
    local vars=(OPENAI_API_KEY ANTHROPIC_API_KEY DEEPSEEK_API_KEY GEMINI_API_KEY
                XAI_API_KEY MOONSHOT_API_KEY DASHSCOPE_API_KEY ZAI_API_KEY
                OPENAI_COMPATIBLE_API_KEY GH_TOKEN)
    local -A label=( [XAI_API_KEY]=Grok [MOONSHOT_API_KEY]=Kimi [DASHSCOPE_API_KEY]=Qwen
                     [ZAI_API_KEY]=Z.ai
                     [GEMINI_API_KEY]=Gemini [OPENAI_API_KEY]=OpenAI
                     [ANTHROPIC_API_KEY]=Anthropic [DEEPSEEK_API_KEY]=DeepSeek
                     [OPENAI_COMPATIBLE_API_KEY]=generic [GH_TOKEN]=GitHub )
    printf 'secrets dir: %s%s\n' "$PALADIN_SECRETS_DIR" \
        "$([ -d "$PALADIN_SECRETS_DIR" ] || echo '  (ABSENT — bind mount not active)')"
    local v val n=0
    for v in "${vars[@]}"; do
        val="${!v:-}"
        if [ -n "$val" ]; then
            printf '  %-26s %-10s set (%s chars)\n' "$v" "${label[$v]}" "${#val}"
            n=$((n+1))
        else
            printf '  %-26s %-10s -\n' "$v" "${label[$v]}"
        fi
    done
    printf '%d of %d credentials available\n' "$n" "${#vars[@]}"
    [ "$n" -gt 0 ]
}
