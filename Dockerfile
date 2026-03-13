FROM ubuntu:latest
LABEL authors="pv"

ENTRYPOINT ["top", "-b"]