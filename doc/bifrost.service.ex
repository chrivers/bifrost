[Unit]
Description=Bifrost Bridge
After=network.target

[Service]
Type=simple

# Make it possible for unprivileged processes to bind to low ports (< 1024)
# This is needed to run port 80 + 443 without being root.
AmbientCapabilities=CAP_NET_BIND_SERVICE

# If bifrost should fail for some reason, wait 20s and restart it,
# no matter the cause
Restart=always
RestartSec=20s

# To use these settings, create a bifrost user + group:
#
#     adduser --group bifrost --system bifrost
#
User=bifrost
Group=bifrost

# This assumes you want to run the bifrost server in:
#
#     /data/bifrost/
#
# with the executable at:
#
#     /data/bifrost/bifrost
#
WorkingDirectory=/data/bifrost
ExecStart=/data/bifrost/bifrost

[Install]
WantedBy=multi-user.target
