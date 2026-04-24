(use mise for tools?)
apt-get update
apt-get install curl

Install tailscale on mainstream linux distros

curl -fsSL https://tailscale.com/install.sh | sh

Start tailscaled:
tailscaled --tun=userspace-networking --socks5-server=localhost:1055 &
(need to setup logging)

authenticate tailscale
tailscale up --ssh --json
(require manually auth) -- but that's maybe ok, though not sure how this'll work durring docker run

docker run --name mycontainer myimage | while IFS= read -r line; do
echo "$line"
    if [[ "$line" == _"Server started"_ ]]; then
echo "Detected trigger phrase, detaching..."
break
fi
done

Get tailscale ip
tailscale ip

## TODO

for ghostty to work well, need to run this:
infocmp -x xterm-ghostty | ssh k@<kitchen ip> -- tic -x -
