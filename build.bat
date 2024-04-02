@echo off

echo "Build en cours ..."
cargo build --target armv7-unknown-linux-gnueabihf && (

echo "Envoi du build ..."
scp -B target/armv7-unknown-linux-gnueabihf/debug/telemetrie master@192.168.1.4:telemetrie && (
echo "Changement de permission ..."
ssh master@192.168.1.4 "chmod +x telemetrie" && (
echo "Terminée."
) || (
    echo "Echec lors de la modification des permissions"
)
) || (
    echo "Echec lors de l'envoi du build"
)
) || (
    echo "Echec lors de la compilation"
)
