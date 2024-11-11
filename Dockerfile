# Utilise une base "alpine"
FROM alpine:3.10

# Récupére le binaire
COPY target/armv7-unknown-linux-gnueabihf/release/build/voiturerc /voiturerc

# Rend executable le binaire
CMD [ "chmod +X voiturerc" ]

# Execute le code
ENTRYPOINT ["/voiturerc"]
