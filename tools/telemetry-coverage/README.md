# Telemetry coverage

| Entry point | What it does |
|---|---|
| `abilities.py` | The ability telemetry coverage gate (ability-mechanics AB-C7): crosses every ability method in the dispatch tables with the server receipt and send rows and the client hooks, and writes [telemetry-coverage.md](../../docs/analysis/ability-mechanics/telemetry-coverage.md). `--check` exits 1 on drift or on an empty cell with no listed exception |

```powershell
python tools/telemetry-coverage/abilities.py          # rewrite the matrix
python tools/telemetry-coverage/abilities.py --check  # what CI runs
python -m unittest discover -s tools/telemetry-coverage -p "test_*.py"
```

CI runs both in `test.yml`. Stock Python 3.
