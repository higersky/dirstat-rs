# dirstat && home-sizes-prom

## home-sizes-prom

A textfile collector for [node-exporter](https://github.com/prometheus/node_exporter#textfile-collector). It outputs the summarized sizes of subdirectories under a given folder (default: `/home`) in Prometheus exporter metrics format.

It provides optional cache mechanism based on modification date of files. It can be used by a metric monitor to quickly estimate the used space without scanning the whole disk every time. You can tune the analyzing depth and threshold of durations to make it more accurate.

### Usage

#### Analyze /home  

        $ home-sizes-prom

#### Analyze /data

        $ home-sizes-prom /data

#### Analyze /home with cache 

        $ home-sizes-prom /home -c /var/lib/home-sizes/home.msgpack

#### Analyze /home with cache, depth limit and reliable estimation duration
        # Set depth=3 and regard those folders modified 1 months ago in cache as reliable sizes
        $ home-sizes-prom /home -c /var/lib/home-sizes/home.msgpack -d 3 -t 30



## dirstat-rs

Fast, cross-platform disk usage CLI

This fork fixed duplicate size counts of hardlinks by file id deduplication (Linux inodes, Windows dwVolumeSerialNumber, nFileIndexHigh, nFileIndexLow). 

![Language](https://img.shields.io/badge/language-rust-orange)
![Platforms](https://img.shields.io/badge/platforms-Windows%2C%20macOS%20and%20Linux-blue)
![License](https://img.shields.io/github/license/scullionw/dirstat-rs)

![](demo/ds_demo.gif)

### Usage

#### Current directory
    
        $ ds
    
#### Specific path
 
        $ ds PATH

#### Choose depth
 
        $ ds -d 3

#### Show apparent size on disk

        $ ds -a PATH

#### Override minimum size threshold

        $ ds -m 0.2 PATH
