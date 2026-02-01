#!/bin/bash
# Execute SQL query in TiDB using host mysql client

set -e

if [ $# -lt 1 ]; then
    echo "Usage: $0 <sql_query>"
    echo "Example: $0 \"SELECT * FROM users WHERE email='test@example.com';\""
    exit 1
fi

SQL_QUERY="$1"

# Check if mysql client is available
if ! command -v mysql &> /dev/null; then
    echo "Error: mysql client not found on host"
    echo "Please install mysql client: brew install mysql-client"
    exit 1
fi

mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e "$SQL_QUERY"
