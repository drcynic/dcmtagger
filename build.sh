go build -ldflags "-X main.commit=`git rev-parse --short HEAD` -X main.version=`git describe --tags --abbrev=0`"

