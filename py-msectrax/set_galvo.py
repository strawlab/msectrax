import requests

# url = "http://127.0.0.1:8080/callback"
url = "http://127.0.0.1:5050/callback"

set_galvos_data = {"SetGalvos": [0,0]}
r = requests.post(url=url, json=set_galvos_data)
r.raise_for_status()
print(r.json())

query_analog_data = "QueryAnalog"
r = requests.post(url=url, json=query_analog_data)
r.raise_for_status()
print(r.json())
