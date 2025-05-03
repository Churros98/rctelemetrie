# Utilise une base "alpine"
FROM alpine:3.21.3

# Récupére le binaire
COPY target/armv7-unknown-linux-gnueabihf/debug/build/voiturerc /entrypoint.sh

# Execute le code
ENTRYPOINT ["/voiturerc"]