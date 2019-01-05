# dirstat-rs

    A toy disk usage cli similar to windirstat

    USAGE:
        dirstat-rs [OPTIONS] [target_dir]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -d <max_depth>          Maximum recursion depth in directory [default: 3]
        -m <min_percent>        Threshold that determines if entry is worth being shown. Between 0-100 % of dir size.
                                [default: 1]

    ARGS:
        <target_dir>