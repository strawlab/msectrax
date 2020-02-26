import requests
import time

url = "http://127.0.0.1:8080/callback"

count = 0
while 1:
    data = {"EchoRequest8": [1, 2, 3, 4, 5, 6, 7, count]}
    count += 1
    count %= 256

    r = requests.post(url=url, json=data)
    r.raise_for_status()
    print(r.json())
    time.sleep(0.01)
