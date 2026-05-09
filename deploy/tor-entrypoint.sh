#!/bin/sh
set -e
# Tor is installed in the image at build time (avoids apk + DNS on every start — important on Docker Desktop / flaky DNS).
mkdir -p /var/lib/tor/hidden_service/
{
  echo "HiddenServiceDir /var/lib/tor/hidden_service/"
  echo "HiddenServicePort 80 127.0.0.1:8080"
} > /etc/tor/torrc
chmod 700 /var/lib/tor/hidden_service/
chown -R tor:tor /var/lib/tor/
exec su -s /bin/sh tor -c "exec tor -f /etc/tor/torrc"
