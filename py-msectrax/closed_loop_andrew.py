import requests
import time
import argparse

parser = argparse.ArgumentParser()
parser.add_argument("head_stage_name")
args = parser.parse_args()
headstage=args.head_stage_name

if headstage=="headstage1":
    url = "http://127.0.0.1:5050/callback"

    # Calib file: 2019-07-19T16.45.16H1.csv
    # Central region: {'dac1': (-12600, -11700), 'dac2': (8600, 9900)}
    # Angle1 (DAC1) = -0.0104477*ADC1 + 0.679328*ADC2 + -13388.9
    # Angle2 (DAC2) = 0.825076*ADC1 + 0.103173*ADC2 + 6524.86

    data = {
        "SetState": {
            "mode": {"ClosedLoop":"Proportional"},
            "cl_period": 1, # integer range 1 (no delay) to u32 maximum
            "dac1_initial": -11093,
            "dac2_initial": 8853,
            "dac1_angle_func": {
                "adc1_gain": -0.010447744188452558,
                "adc2_gain": 0.6793280985084404,
                "offset": -13388.874373535962,
            },
            "dac2_angle_func": {
                "adc1_gain": 0.8250760411287147,
                "adc2_gain": 0.10317298178375989,
                "offset": 6524.860850191762,
            },
            "dac1_angle_gain": -0.02,
            "dac2_angle_gain": -0.02,
            "dac1_min": -32768,
            "dac1_max": 32767,
            "dac2_min": -32768,
            "dac2_max": 32767,
        }
    }



    
if headstage=="headstage2":
    url = "http://127.0.0.1:8080/callback"
    # Calib file: 2019-07-04T15.16.49H2.csv
    # Central region: {'dac1': (-3600, -2700), 'dac2': (-6900, -5600)}
    # Angle1 (DAC1) = -0.0638002*ADC1 + 0.587315*ADC2 + -4811.11
    # Angle2 (DAC2) = 0.946383*ADC1 + -0.0991378*ADC2 + -7760.78

    data = {
        "SetState": {
            "mode": {"ClosedLoop":"Proportional"},
            "cl_period": 1, # integer range 1 (no delay) to u32 maximum
            "dac1_initial": -4386,
            "dac2_initial": -57,
            "dac1_angle_func": {
                "adc1_gain": -0.0638002222554458,
                "adc2_gain": 0.5873149806393063,
                "offset": -4811.105807700699,
            },
            "dac2_angle_func": {
                "adc1_gain": 0.9463829955512771,
                "adc2_gain": -0.09913784171019957,
                "offset": -7760.781758781409,
            },
            "dac1_angle_gain": -0.02,
            "dac2_angle_gain": -0.02,
            "dac1_min": -32768,
            "dac1_max": 32767,
            "dac2_min": -32768,
            "dac2_max": 32767,
        }
    }






r = requests.post(url=url, json=data)
r.raise_for_status()
start_time=time.monotonic_ns()
start_time2=time.monotonic_ns()
actual_cycles1 = 0
while 1:
    r = requests.post(url=url, json="QueryState")
    r.raise_for_status()
    state = r.json()['EchoState']
    # print(state)
    elapsed=time.monotonic_ns()-start_time # num nanoseconds elapsed (int)

    cycles_per_sec = state['cl_cycles'] / (elapsed*1e-9)
    cl_cycles_per_second = cycles_per_sec / float(state['inner']['cl_period'])
    loop_time = 1.0/cl_cycles_per_second
    print('cycle_rate: {:1g} Hz, cl_rate: {:1g} Hz, loop_time: {:1g} usec'.format(cycles_per_sec, cl_cycles_per_second, loop_time*1e6))
    print('    ADC1: {: 8d}     ADC2: {: 8d}       DAC1: {: 8d}     DAC2: {: 8d}'.format(state['adc1'], state['adc2'], state['dac1'], state['dac2']))
    time.sleep(1.0)
