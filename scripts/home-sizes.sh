#!/bin/sh

[ -d /var/lib/home-sizes ] || mkdir /var/lib/home-sizes
[ -d /var/lib/prometheus/node-exporter ] || exit

OUT_HOME_FILE=/var/lib/prometheus/node-exporter/home_sizes.prom
home-sizes-prom -d 3 -t 30 -p 365 -c /var/lib/home-sizes/home.msgpack > "$OUT_HOME_FILE"
