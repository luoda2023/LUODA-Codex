import sys, subprocess
sys.stdout.reconfigure(encoding="utf-8")
result = subprocess.run(["git","show","ff64b5b:apps/luoda-codex-manager/src/App.tsx"],capture_output=True)
text = result.stdout.decode("utf-8")



# Verify
v = text
print("Route clean:", "zedRemote" not in v.split("type Route")[1])
print("C++ replaced:", "C++" not in v)
print("Restart clean:", "重启 Luoda-Codex" not in v)
print("BigPizzaV3 clean:", "BigPizzaV3" not in v)
print("CodexPlusPlus clean:", "CodexPlusPlus" not in v)
print("GitHub Release clean:", "GitHub Release" not in v)
print("SCRIPT_MARKET clean:", "SCRIPT_MARKET" not in v)

# Show AboutScreen
import subprocess
result = subprocess.run(["git","show","ff64b5b:apps/luoda-codex-manager/src/App.tsx"],capture_output=True)
text = result.stdout.decode("utf-8")
print("OK: verify script ready")
