"""Routing tests for egress_proxy.py's node role. Stdlib only:

    python -m unittest tools/ci-runners/test_egress_proxy.py
"""

import asyncio
import importlib.util
import ipaddress
import pathlib
import re
import socket
import types
import unittest
from unittest import mock

_spec = importlib.util.spec_from_file_location(
    "egress_proxy", pathlib.Path(__file__).with_name("egress_proxy.py"))
egress_proxy = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(egress_proxy)

# The stamp behind productionresultssa9/16/17/18 rotates these; the colo
# reaches the two 20.150 addresses but not 52.239.172.36 (README, "Colo egress").
STAMP = ["52.239.172.36", "20.150.82.228", "20.150.88.228"]
UNREACHABLE = ["52.239.172.36", "57.150.86.161"]
HERE = pathlib.Path(__file__).parent


def cidrs_after(flag, text):
    """The CIDRs that follow `flag` on the line that has it."""
    line = next(l for l in text.splitlines() if flag in l)
    return re.findall(r"\d+\.\d+\.\d+\.\d+/\d+", line.split(flag, 1)[1])


def configured_ranges():
    """Where the ranges are set: the Linux nodes, node3 and the exit."""
    return {
        "compose.yml --via-dst": cidrs_after("--via-dst", (HERE / "compose.yml").read_text()),
        "windows/setup.ps1 --via-dst": cidrs_after("--via-dst", (HERE / "windows" / "setup.ps1").read_text()),
        "start-exit.ps1 AllowDst": cidrs_after("$AllowDst =", (HERE / "start-exit.ps1").read_text()),
    }


class ConfiguredRanges(unittest.TestCase):
    def test_every_proxy_role_covers_the_unreachable_storage_addresses(self):
        for where, ranges in configured_ranges().items():
            nets = [ipaddress.ip_network(r) for r in ranges]
            for ip in UNREACHABLE:
                with self.subTest(where=where, ip=ip):
                    self.assertTrue(any(ipaddress.ip_address(ip) in n for n in nets), ranges)


class NodeRouting(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.seen = []

        async def upstream(reader, writer):
            self.seen.append((await reader.readline()).decode())
            while (await reader.readline()) not in (b"\r\n", b""):
                pass
            writer.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
            await writer.drain()
            writer.close()

        self.exit = await asyncio.start_server(upstream, "127.0.0.1", 0)
        exit_port = self.exit.sockets[0].getsockname()[1]
        args = types.SimpleNamespace(
            allow_src=None, allow_dst=None, via=f"127.0.0.1:{exit_port}",
            via_dst=configured_ranges()["compose.yml --via-dst"])
        self.node = await asyncio.start_server(egress_proxy.Proxy(args).handle, "127.0.0.1", 0)
        self.node_port = self.node.sockets[0].getsockname()[1]

    async def asyncTearDown(self):
        for s in (self.node, self.exit):
            s.close()
            await s.wait_closed()

    async def connect_through_node(self, host, addrs, port=443, timeout=5):
        async def fake_getaddrinfo(h, p, **kw):
            return [(socket.AF_INET, socket.SOCK_STREAM, 6, "", (a, p)) for a in addrs]

        loop = asyncio.get_running_loop()
        with mock.patch.object(loop, "getaddrinfo", fake_getaddrinfo):
            r, w = await asyncio.open_connection("127.0.0.1", self.node_port)
            w.write(f"CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n".encode())
            await w.drain()
            status = await asyncio.wait_for(r.readline(), timeout)
            w.close()
            return status.decode()

    async def test_a_stamp_with_any_unreachable_address_goes_via_the_exit(self):
        host = "productionresultssa17.blob.core.windows.net"
        # Each rotation of the A records, the unreachable one first or not.
        for i in range(len(STAMP)):
            order = STAMP[i:] + STAMP[:i]
            status = await self.connect_through_node(host, order)
            self.assertIn(" 200 ", status, order)
        self.assertEqual(len(self.seen), len(STAMP))
        self.assertTrue(all(line.startswith(f"CONNECT {host}:443 ") for line in self.seen))

    async def test_a_host_outside_the_via_ranges_connects_directly(self):
        accepted = []

        async def direct(reader, writer):
            accepted.append(True)
            writer.close()

        server = await asyncio.start_server(direct, "127.0.0.1", 0)
        port = server.sockets[0].getsockname()[1]
        try:
            # 127.0.0.1 is outside --via-dst.
            status = await self.connect_through_node("example.invalid", ["127.0.0.1"], port)
        finally:
            server.close()
            await server.wait_closed()
        self.assertIn(" 200 ", status)
        self.assertEqual(accepted, [True])
        self.assertEqual(self.seen, [])

    async def test_a_failed_direct_connect_names_the_address_and_error(self):
        # A closed port; Windows retries a refused SYN for about 2 s.
        with self.assertLogs("egress-proxy", "WARNING") as logs:
            status = await self.connect_through_node("example.invalid", ["127.0.0.1"], 9, timeout=30)
        self.assertIn(" 502 ", status)
        self.assertIn("(127.0.0.1) failed: ConnectionRefusedError", logs.output[0])


if __name__ == "__main__":
    unittest.main()
