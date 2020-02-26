import requests
import numpy as np
import time
import datetime
import os
import json
from tzlocal import get_localzone # $ pip install tzlocal
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("head_stage_name")
parser.add_argument("dac1_min", type=int, help="low limit of the scan")
parser.add_argument("dac1_max", type=int, help="high limit of the scan")
parser.add_argument("dac1_step", type=int, help="scaning step")
parser.add_argument("dac2_min", type=int, help="low limit of the scan")
parser.add_argument("dac2_max", type=int, help="high limit of the scan")
parser.add_argument("dac2_step", type=int, help="scaning step")
args = parser.parse_args()
headstage=args.head_stage_name
scan1_min=args.dac1_min
scan1_max=args.dac1_max
scan1_step=args.dac1_step
scan2_min=args.dac2_min
scan2_max=args.dac2_max
scan2_step=args.dac2_step

if headstage=="headstage1":
    url = "http://127.0.0.1:5050/callback"
    # url = "http://127.0.0.1:5050/callback"

    save_cols = ('timestamp','dac1','dac2','adc1','adc2')

    outputFilePath = os.path.join(os.path.dirname(__file__),
                    datetime.datetime.now().strftime("%Y-%m-%dT%H.%M.%S") + "H1.csv")
if headstage=="headstage2":
    url = "http://127.0.0.1:8080/callback"

    save_cols = ('timestamp','dac1','dac2','adc1','adc2')

    outputFilePath = os.path.join(os.path.dirname(__file__),
                    datetime.datetime.now().strftime("%Y-%m-%dT%H.%M.%S") + "H2.csv")
outputfile = open(outputFilePath, mode='wb')

# write a comment line
now = datetime.datetime.now(get_localzone())
outdatastr = "# saved by qpd_scan.py at %s"%(now.isoformat('T'),) + "\n"
outputfile.write(outdatastr.encode())

# write CSV header

outdatastr = ",".join(colname for colname in save_cols) + "\n"
outputfile.write(outdatastr.encode())

set_galvos_data = {"SetGalvos": [0x0, 0x0]}
start_time = time.monotonic_ns()
while (1):
    for vdac1 in np.arange (scan1_min,scan1_max,scan1_step):
        for vdac2 in np.arange (scan2_min,scan2_max,scan2_step):

            set_galvos_data = {"SetGalvos": [int(vdac1), int(vdac2)]}
            r = requests.post(url=url, json=set_galvos_data)
            r.raise_for_status()
            #print(r.json())
            query_analog_data = "QueryState"
            r = requests.post(url=url, json=query_analog_data)
            r.raise_for_status()
            #print(r.json())
            # test1= r.json()['EchoState']['adc1']
            # print(test1)
            # test1.dtype=np.float32
            # print(test1)

            t=time.monotonic_ns()-start_time # num nanoseconds elapsed (int)

            state = r.json()['EchoState']
            state['timestamp'] = t

            outdatastr = ",".join(str(state[colname]) for colname in save_cols) + "\n"
            outputfile.write(outdatastr.encode())
            print(outdatastr)
            # time.sleep(0.1)
