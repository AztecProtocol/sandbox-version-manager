#!/usr/bin/env bash
set -e


AZTEC_HOME=${AZTEC_HOME-"$HOME/.aztec"}
AZTEC_BIN_DIR="$AZTEC_HOME/bin"
SANDBOX_CLI_BIN_PATH="$AZTEC_BIN_DIR/aztec-sandbox"

main() {
    echo Installing aztec version manager...

    # Create the .aztec bin directory and aztec binary if it doesn't exist.
    mkdir -p $AZTEC_BIN_DIR
    chmod +x $SANDBOX_CLI_BIN_PATH

    # Store the correct profile file (i.e. .profile for bash or .zshrc for ZSH).
    case $SHELL in
    */zsh)
      PROFILE=$HOME/.zshrc
      PREF_SHELL=zsh
      ;;
    */bash)
      PROFILE=$HOME/.bashrc
      PREF_SHELL=bash
      ;;
    */fish)
      PROFILE=$HOME/.config/fish/config.fish
      PREF_SHELL=fish
      ;;
    */ash)
      PROFILE=$HOME/.profile
      PREF_SHELL=ash
      ;;
    *)
      echo "aztec-sandbox installer: could not detect shell, manually add ${AZTEC_BIN_DIR} to your PATH."
      exit 1
      ;;
    esac



    SANDBOX_CLI_REPO=${SANDBOX_CLI_REPO-"AztecProtocol/sandbox-version-manager"}
    PLATFORM="$(uname -s)"
    case $PLATFORM in
        Linux)
        PLATFORM="unknown-linux-gnu"
        CONFIG_DIR=${XDG_CONFIG_HOME-"$HOME/.config"}
        ;;
        Darwin)
        PLATFORM="apple-darwin"
        CONFIG_DIR=${XDG_CONFIG_HOME-"$HOME/Library/Application Support"}
        ;;
        *)
        err "unsupported platform: $PLATFORM"
        ;;
    esac

    # Fetch system's architecture.
    ARCHITECTURE="$(uname -m)"

    # Align ARM naming for release fetching.
    if [ "${ARCHITECTURE}" = "arm64" ]; then
        ARCHITECTURE="aarch64" # Align release naming.
    fi

    # Reject unsupported architectures.
    if [ "${ARCHITECTURE}" != "x86_64" ] && [ "${ARCHITECTURE}" != "aarch64" ]; then
        err "unsupported architecure: $ARCHITECTURE-$PLATFORM"
    fi

    # Compute the URL of the release tarball in the aztec-sandbox repository.
    say "installing aztec-sandbox (latest)"
    RELEASE_URL="https://github.com/${SANDBOX_CLI_REPO}/releases/download/nightly"

    BIN_TARBALL_URL="${RELEASE_URL}/aztec-sandbox-${ARCHITECTURE}-${PLATFORM}.tar.gz"

    # Download the binaries tarball and unpack it into the .aztec bin directory.
    say $BIN_TARBALL_URL
    say "downloading latest aztec-sandbox to '$AZTEC_BIN_DIR'"
    ensure curl -# -L $BIN_TARBALL_URL | tar -xzC $AZTEC_BIN_DIR

    # Depending on the user's OS we want to add .aztec/bin to their PATH
    if [[ ":$PATH:" != *":${AZTEC_BIN_DIR}:"* ]]; then
      # Add the aztec directory to the path and ensure the old PATH variables remain.
      echo >>$PROFILE && echo "export AZTEC_HOME=\"$AZTEC_HOME\"" >>$PROFILE
      echo >>$PROFILE && echo "export PATH=\"\$PATH:\$AZTEC_HOME/bin\"" >>$PROFILE
    fi

    echo && echo "Detected your preferred shell is ${PREF_SHELL} and added aztec-sandbox to PATH. Run 'source ${PROFILE}' or start a new terminal session to use aztec-sandbox."

}


say() {
  printf 'aztec-sandbox installer: %s\n' "$1"
}

warn() {
  say "warning: ${1}" >&2
}

err() {
  say "$1" >&2
  exit 1
}

need_cmd() {
  if ! check_cmd "$1"; then
    err "need '$1' (command not found)"
  fi
}

check_cmd() {
  command -v "$1" >/dev/null 2>&1
}

# Run a command that should never fail. If the command fails execution
# will immediately terminate with an error showing the failing
# command.
ensure() {
  if ! "$@"; then err "command failed: $*"; fi
}

main "$@" || exit 1