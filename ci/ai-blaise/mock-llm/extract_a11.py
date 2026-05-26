import sys, json, re
m = re.search(r"\{.*\}", sys.stdin.read(), re.S)
if m:
    obj = json.loads(m.group(0))
    field = sys.argv[1]
    print(obj.get(field, ""))
else:
    print("")
