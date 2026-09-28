tf() {
  if [ "$#" -eq 0 ]; then
    command tf
    return
  fi
  first="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  case "$first" in
    cd|chdir|перейди|перейти|зайди|go|goto|enter|entra|entrar|gehe)
      shift
      dest="$(command tf --resolve-dir "$@")" || return $?
      builtin cd "$dest" || return $?
      pwd
      ;;
    *)
      command tf "$@"
      ;;
  esac
}
