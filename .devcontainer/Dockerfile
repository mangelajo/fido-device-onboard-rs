FROM fedora:latest

RUN bash -c "$(curl -fsSL "https://raw.githubusercontent.com/microsoft/vscode-dev-containers/main/script-library/common-redhat.sh")" -- "true" "vscode" "1000" "1000" "true"

RUN dnf install -y \
    sudo git cargo rust rust-src openssl-devel clippy rustfmt golang \
    && dnf clean all

USER vscode
