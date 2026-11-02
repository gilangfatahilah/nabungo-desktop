#!/usr/bin/env bash
# Firewall test (PRD §8: "0 request keluar"). Jalankan binary release di network namespace yang
# hanya punya `lo` + interface dummy (default route + DNS diarahkan ke sana), rekam semua paket IP
# dengan tcpdump, dan pastikan jumlahnya 0. App memakai protokol tauri://, jadi paket di `lo` pun
# dihitung. Fixture memakai konfigurasi default (tanpa auto-capture email `EML-*` dan tanpa cek
# update otomatis `update.auto_check`, keduanya opt-in):
# fitur jaringan opt-in tidak boleh membuka koneksi selama belum dikonfigurasi.
#
# Pakai: scripts/firewall-test.sh <app-bin> <seed_fixture-bin>
#   FW_SELFTEST=1  ganti app dengan lookup DNS (harus exit 1: membuktikan tangkapan bekerja)
# Exit: 0 = 0 paket, 1 = ada paket, 2 = app tidak sampai dasbor / setup gagal.
# Prasyarat: sudo, ip (iproute2), tcpdump, xvfb-run, dbus-run-session.

set -euo pipefail

APP_BIN=$(realpath "${1:?pakai: firewall-test.sh <app-bin> <seed-bin>}")
SEED_BIN=$(realpath "${2:?pakai: firewall-test.sh <app-bin> <seed-bin>}")
NS=nbg-fw
RUN_USER=$(id -un)
WORK=$(mktemp -d)
TCPDUMP_PID=

cleanup() {
  set +e
  sudo ip netns pids "$NS" 2>/dev/null | xargs -r sudo kill 2>/dev/null
  [ -n "$TCPDUMP_PID" ] && wait "$TCPDUMP_PID" 2>/dev/null
  sudo ip netns del "$NS" 2>/dev/null
  sudo rm -rf "/etc/netns/$NS"
  rm -rf "$WORK"
}
trap cleanup EXIT

fail_setup() { echo "firewall test: $*" >&2; exit 2; }

# Folder portable berisi fixture: tidak menyentuh data user.
cp "$APP_BIN" "$WORK/nabungo"
touch "$WORK/portable"
"$SEED_BIN" "$WORK/data" --txns 2000 >/dev/null || fail_setup "seed fixture gagal"

sudo ip netns del "$NS" 2>/dev/null || true
sudo ip netns add "$NS"
# Tanpa IPv6: kernel mengirim router solicitation/DAD sendiri dari interface baru (paket bukan dari
# app). Namespace tidak punya route IPv6, jadi koneksi v6 app tetap gagal tanpa paket.
sudo ip netns exec "$NS" sysctl -qw net.ipv6.conf.all.disable_ipv6=1 net.ipv6.conf.default.disable_ipv6=1
sudo ip -n "$NS" link set lo up
sudo ip -n "$NS" link add fw0 type dummy
sudo ip -n "$NS" addr add 10.255.0.2/24 dev fw0
sudo ip -n "$NS" link set fw0 up
sudo ip -n "$NS" route add default via 10.255.0.1 dev fw0
# `ip netns exec` me-bind-mount file ini ke /etc/resolv.conf: DNS ikut lewat fw0.
sudo mkdir -p "/etc/netns/$NS"
echo "nameserver 10.255.0.1" | sudo tee "/etc/netns/$NS/resolv.conf" >/dev/null

sudo ip netns exec "$NS" tcpdump -Z root -i any -n -U -w "$WORK/fw.pcap" 'ip or ip6' 2>"$WORK/tcpdump.err" &
TCPDUMP_PID=$!
sleep 1
# Tangkapan yang mati diam-diam akan terbaca "0 paket".
sudo kill -0 "$TCPDUMP_PID" 2>/dev/null || { cat "$WORK/tcpdump.err" >&2; fail_setup "tcpdump tidak berjalan"; }

as_user() { sudo ip netns exec "$NS" sudo -u "$RUN_USER" env HOME="$HOME" "$@"; }

if [ "${FW_SELFTEST:-}" = 1 ]; then
  as_user getent hosts example.com >/dev/null || true
  sleep 1
else
  # WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS: lihat R6 di plan Fase 8d (hanya di test, sandbox
  # bubblewrap gagal di beberapa runner CI; yang diukur tetap trafik jaringan).
  as_user GDK_BACKEND=x11 ${FW_WEBKIT_NO_SANDBOX:+WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1} \
    xvfb-run -a dbus-run-session -- "$WORK/nabungo" >"$WORK/app.out" 2>&1 &
  APP_PID=$!
  for _ in $(seq 60); do
    grep -rqs 'dasbor pertama tampil' "$WORK/data/logs" && break
    kill -0 "$APP_PID" 2>/dev/null || break
    sleep 1
  done
  if ! grep -rqs 'dasbor pertama tampil' "$WORK/data/logs"; then
    echo "--- app output ---" >&2; cat "$WORK/app.out" >&2
    echo "--- log ---" >&2; cat "$WORK"/data/logs/* >&2 2>/dev/null || true
    fail_setup "app tidak sampai dasbor dalam 60 s"
  fi
  # Beri waktu request malas dan backup otomatis startup.
  sleep 15
fi

# Hentikan app dan tcpdump sekaligus (-U: setiap paket sudah tertulis ke file).
sudo ip netns pids "$NS" | xargs -r sudo kill 2>/dev/null || true
wait "$TCPDUMP_PID" 2>/dev/null || true
TCPDUMP_PID=

PACKETS=$(tcpdump -r "$WORK/fw.pcap" -n 2>/dev/null) || fail_setup "hasil tangkapan tidak bisa dibaca"
COUNT=$(printf '%s' "$PACKETS" | grep -c . || true)
if [ "$COUNT" -gt 0 ]; then
  echo "firewall test: $COUNT paket keluar" >&2
  printf '%s\n' "$PACKETS" | head -50 >&2
  exit 1
fi
echo "firewall test: 0 paket"
