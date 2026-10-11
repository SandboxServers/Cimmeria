# The routing HTTPS proxy the runners use (egress_proxy.py, node role).
FROM python:3.13-alpine
COPY egress_proxy.py /egress_proxy.py
USER nobody
ENTRYPOINT ["python", "-I", "-u", "/egress_proxy.py"]