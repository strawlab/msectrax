import requests
import numpy as np
import time
import datetime
import os
import json
from tzlocal import get_localzone # $ pip install tzlocal

url = "http://127.0.0.1:8080/callback"

save_cols = ('timestamp','dac1','dac2','adc1','adc2')

outputFilePath = os.path.join(os.path.dirname(__file__),
                 datetime.datetime.now().strftime("log-%Y-%m-%dT%H.%M.%S") + ".csv")
outputfile = open(outputFilePath, mode='wb')

# write a comment line
now = datetime.datetime.now(get_localzone())
outdatastr = "# saved by log_data_slow.py at %s"%(now.isoformat('T'),) + "\n"
outputfile.write(outdatastr.encode())

# write CSV header
outdatastr = ",".join(colname for colname in save_cols) + "\n"
outputfile.write(outdatastr.encode())

# loop forever and save data
start_time = time.monotonic_ns()
while True:
    r = requests.post(url=url, json="QueryState")
    r.raise_for_status()
    t=time.monotonic_ns()-start_time # num nanoseconds elapsed (int)

    state = r.json()['EchoState']
    state['timestamp'] = t

    outdatastr = ",".join(str(state[colname]) for colname in save_cols) + "\n"
    outputfile.write(outdatastr.encode())
    print(outdatastr.strip())
    # time.sleep(0.1)
