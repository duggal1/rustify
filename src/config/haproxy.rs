fn generate_haproxy_config(mode: &str) -> String {
    format!(r#"global
    maxconn 500000
    ssl-default-bind-ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256
    ssl-default-bind-options no-sslv3 no-tlsv10 no-tlsv11
    tune.ssl.default-dh-param 2048
    stats socket /var/run/haproxy.sock mode 600 expose-fd listeners
    stats timeout 30s
    cpu-map auto:1/1-4 0-3
    numa-cpu-mapping
    tune.bufsize 32768
    tune.maxrewrite 8192
    tune.ssl.cachesize 100000

defaults
    mode http
    option httplog
    option dontlognull
    option http-server-close
    option redispatch
    retries 3
    timeout http-request 10s
    timeout queue 20s
    timeout connect 10s
    timeout client 1h
    timeout server 1h
    timeout http-keep-alive 10s
    timeout check 10s
    maxconn 100000

frontend main
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/cert.pem alpn h2,http/1.1
    
    # Security headers
    http-response set-header Strict-Transport-Security "max-age=63072000"
    http-response set-header X-Frame-Options "DENY"
    http-response set-header X-Content-Type-Options "nosniff"
    
    # WAF rules
    filter spoe engine modsecurity
    tcp-request inspect-delay 5s
    tcp-request content accept if { req.len gt 0 }
    
    # Rate limiting
    stick-table type ip size 1m expire 1h store gpc0,http_req_rate(10s)
    tcp-request content track-sc0 src
    tcp-request content reject if { sc_http_req_rate(0) gt 100 }
    
    # Advanced routing
    use_backend bun_backend if { path_beg /api }
    use_backend static_backend if { path_end .jpg .png .css .js }
    default_backend dynamic_backend

backend bun_backend
    balance leastconn
    option httpchk GET /health HTTP/1.1\r\nHost:\ localhost
    http-check expect status 200
    server bun1 127.0.0.1:3000 check ssl verify none maxconn 50000
    server bun2 127.0.0.1:3001 check ssl verify none maxconn 50000
    compression algo gzip
    compression type text/html text/plain application/json
    "#)
} 