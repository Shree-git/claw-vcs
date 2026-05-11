[Unit]
Description=Claw Daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User={{ .User }}
Group={{ .Group }}
WorkingDirectory={{ .WorkingDirectory }}
EnvironmentFile=-{{ .EnvironmentFile }}
ExecStart={{ .BinaryPath }} daemon --listen {{ .ListenAddr }} --health-listen {{ .HealthListenAddr }} {{ .ExtraArgs }}
Restart=on-failure
RestartSec=5s
TimeoutStartSec=30s
TimeoutStopSec=30s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ProtectControlGroups=true
ProtectKernelModules=true
ProtectKernelTunables=true
LockPersonality=true
MemoryDenyWriteExecute=true
ReadWritePaths={{ .StateDirectory }}

[Install]
WantedBy=multi-user.target
