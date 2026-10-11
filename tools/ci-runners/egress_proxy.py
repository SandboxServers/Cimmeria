#!/usr/bin/env python3
"""A small HTTPS (CONNECT-only) proxy with two roles. Stdlib only.

Node role, next to the runners: sends CONNECTs whose destination resolves into
--via-dst through the upstream proxy given by --via, and connects everything else
directly. The colo cannot reach some Azure Storage ranges (README, "Colo egress"),
so only those uploads take the detour.

    egress_proxy.py --listen 0.0.0.0:3128 --via EXIT_HOST:3128 --via-dst 57.0.0.0/8

Exit role, on a machine whose internet path works: only accepts clients from
--allow-src and only connects to destinations in --allow-dst, so it is not an
open proxy.

    egress_proxy.py --listen VPN_ADDR:3128 --allow-src 10.0.30.0/24 --allow-dst 57.0.0.0/8

Plain-HTTP proxying is not supported; the runners set only https_proxy.
"""

import argparse
import asyncio
import ipaddress
import logging
import socket

log = logging.getLogger("egress-proxy")
TIMEOUT = 20


def nets(values):
    return [ipaddress.ip_network(v, strict=False) for v in values or []]


def inside(ip, networks):
    addr = ipaddress.ip_address(ip)
    return any(addr in n for n in networks)


async def read_head(reader):
    """Request or status line plus headers, up to the blank line."""
    first = await asyncio.wait_for(reader.readline(), TIMEOUT)
    while True:
        line = await asyncio.wait_for(reader.readline(), TIMEOUT)
        if line in (b"\r\n", b"\n", b""):
            return first


async def pipe(reader, writer):
    try:
        while data := await reader.read(65536):
            writer.write(data)
            await writer.drain()
    except (ConnectionError, OSError, asyncio.CancelledError):
        pass
    finally:
        try:
            writer.close()
        except OSError:
            pass


async def reply(writer, status):
    writer.write(f"HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".encode())
    try:
        await writer.drain()
    finally:
        writer.close()


class Proxy:
    def __init__(self, args):
        self.allow_src = nets(args.allow_src)
        self.allow_dst = nets(args.allow_dst)
        self.via = args.via
        self.via_dst = nets(args.via_dst)

    async def handle(self, reader, writer):
        peer = writer.get_extra_info("peername")[0]
        target = "?"
        try:
            if self.allow_src and not inside(peer, self.allow_src):
                log.warning("refused client %s (not in --allow-src)", peer)
                return await reply(writer, "403 Forbidden")
            parts = (await read_head(reader)).decode("latin-1").split()
            if len(parts) < 2 or parts[0].upper() != "CONNECT":
                return await reply(writer, "405 Method Not Allowed")
            target = parts[1]
            host, _, port = target.rpartition(":")
            host, port = host.strip("[]"), int(port)

            infos = await asyncio.get_running_loop().getaddrinfo(
                host, port, family=socket.AF_INET, type=socket.SOCK_STREAM)
            ips = list(dict.fromkeys(i[4][0] for i in infos))
            if self.allow_dst:
                ips = [ip for ip in ips if inside(ip, self.allow_dst)]
                if not ips:
                    log.warning("refused %s -> %s (not in --allow-dst)", peer, target)
                    return await reply(writer, "403 Forbidden")

            if self.via and any(inside(ip, self.via_dst) for ip in ips):
                route = f"via {self.via}"
                vhost, _, vport = self.via.rpartition(":")
                up_r, up_w = await asyncio.wait_for(asyncio.open_connection(vhost, int(vport)), TIMEOUT)
                up_w.write(f"CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n".encode())
                await up_w.drain()
                status = (await read_head(up_r)).decode("latin-1")
                if " 200 " not in f"{status} ":
                    log.error("upstream refused %s: %s", target, status.strip())
                    up_w.close()
                    return await reply(writer, "502 Bad Gateway")
            else:
                route = "direct"
                # Connect to the address we checked, not a fresh lookup.
                up_r, up_w = await asyncio.wait_for(asyncio.open_connection(ips[0], port), TIMEOUT)

            log.info("%s -> %s (%s) %s", peer, target, ips[0], route)
            writer.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
            await writer.drain()
            await asyncio.gather(pipe(reader, up_w), pipe(up_r, writer))
        except (OSError, asyncio.TimeoutError, ValueError, UnicodeError) as e:
            log.warning("%s -> %s failed: %s", peer, target, e)
            try:
                await reply(writer, "502 Bad Gateway")
            except OSError:
                pass


async def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--listen", required=True, help="host:port to listen on")
    ap.add_argument("--allow-src", nargs="*", help="client networks allowed (default: any)")
    ap.add_argument("--allow-dst", nargs="*", help="destination networks allowed (default: any)")
    ap.add_argument("--via", help="upstream proxy host:port for --via-dst destinations")
    ap.add_argument("--via-dst", nargs="*", default=[], help="destination networks sent --via")
    ap.add_argument("--log-file", help="log here instead of stderr (needed under pythonw)")
    args = ap.parse_args()
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s",
                        filename=args.log_file)

    host, _, port = args.listen.rpartition(":")
    proxy = Proxy(args)
    server = await asyncio.start_server(proxy.handle, host, int(port))
    log.info("listening on %s (allow-src=%s allow-dst=%s via=%s for %s)",
             args.listen, args.allow_src, args.allow_dst, args.via, args.via_dst)
    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
