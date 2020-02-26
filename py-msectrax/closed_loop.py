import requests
import time

url = "http://127.0.0.1:8080/callback"

data = {
    "SetState": {
        "mode": {"ClosedLoop":"Proportional"},
        "cl_cycles": 0,
        "cl_period": 10, # integer range 1 (no delay) to u32 maximum
        "dac1": 0,
        "dac2": 0,
        "adc1": 0,
        "adc2": 0,
        "dac1_f32": 0.0,
        "dac2_f32": 0.0,
        "dac1_angle_func": {
            "adc1_offset": 0,
            "adc1_gain": 0.0,
            "adc2_offset": -1954,
            "adc2_gain": 1.0/0.238,
        },
        "dac2_angle_func": {
            "adc1_offset": -2065,
            "adc1_gain": 1.0/-0.22,
            "adc2_offset": 0,
            "adc2_gain": 0.0,
        },
        "dac1_angle_gain": 1e-3,
        "dac2_angle_gain": 1e-3,
        "dac1_min": -32768,
        "dac1_max": 32767,
        "dac2_min": -32768,
        "dac2_max": 32767,
    }
}

r = requests.post(url=url, json=data)
r.raise_for_status()
print(r.json())

while 1:
    query_data = "QueryState"
    r = requests.post(url=url, json=query_data)
    r.raise_for_status()
    print(r.json())

    query_data = "QueryActualCycles"
    r = requests.post(url=url, json=query_data)
    r.raise_for_status()
    print(r.json())

    time.sleep(1.0)
    print()
