import requests
import numpy as np
import pandas as pd
import os
import time
import datetime
import json
import keyboard
from sklearn.linear_model import LinearRegression
from sklearn.metrics import mean_squared_error, r2_score
import statistics as stat
from random import *

url = "http://127.0.0.1:8080/callback"
# Set logging file name
outputFilePath = os.path.join(os.path.dirname(__file__),
                 datetime.datetime.now().strftime("%Y-%m-%dT%H.%M.%S") + ".csv")
outputfile = open(outputFilePath, mode='wb')

#load QPD map and estimate the linear functions
data=pd.read_csv('data/2019-04-24T18.00.59.csv',skiprows=1,usecols=[0,1,2,3,4],header=None, names=['t','dac1','dac2','adc1','adc2'])
data1=data.astype(float)
data1.adc1=(1.5-data1.adc1*3.3/4096)/10 #convert to original value (V)
data1.adc2=(1.5-data1.adc2*3.3/4096)/10
#Pointing to central region
data4=data1.copy()
data5 = data4[data4['dac1'] < 1500]
data5 = data5[data5['dac1'] > -2000]
data5 = data5[data5['dac2'] <2500]
data5 = data5[data5['dac2'] >-1000]

# define dac2 vs adc1 function
data6 = data5[data5['dac1'] == 0]
x= np.asarray(data6['adc1'])
x1=x.reshape(x.shape[0],1)
y=np.asarray(data6['dac2'])
y1=y.reshape(y.shape[0],1)
dac2_model=LinearRegression()
dac2_model.fit(x1, y1)
dac2_predicted = dac2_model.predict(x1)
rmse = mean_squared_error(y1, dac2_predicted)
r2 = r2_score(y1,dac2_predicted)
dac2_slope=dac2_model.coef_
dac2_intercept=dac2_model.intercept_
print('dac2 function: dac2=',dac2_model.coef_,'*adc1+', dac2_model.intercept_,'---R2 score: ', r2 )

# define dac1 vs adc2 function
data7 = data5[data5['dac2'] == 1000]
x= np.asarray(data7['adc2'])
x1=x.reshape(x.shape[0],1)
y=np.asarray(data7['dac1'])
y1=y.reshape(y.shape[0],1)
dac1_model=LinearRegression()
dac1_model.fit(x1, y1)
dac1_predicted = dac1_model.predict(x1)
rmse = mean_squared_error(y1, dac1_predicted)
r2 = r2_score(y1,dac1_predicted)
dac1_slope=dac1_model.coef_
dac1_intercept=dac1_model.intercept_
print('dac1 function: dac1=',dac1_model.coef_,'*adc2+', dac1_model.intercept_,'---R2 score: ', r2 )

#Estimate zero level from QPD map
zero_adc1a=(data5[data5['dac2'] ==500])
zero_adc1b=(data5[data5['dac2'] ==1000])
zero_adc2a=(data5[data5['dac1'] ==-500])
zero_adc2b=(data5[data5['dac1'] ==0])
adc1_0=0.5*(stat.mean(zero_adc1a.adc1)+stat.mean(zero_adc1b.adc1))
adc2_0=0.5*(stat.mean(zero_adc2a.adc2)+stat.mean(zero_adc2b.adc2))
dac1_0=-250
dac2_0=750
curr_dac1=-5000
curr_dac2=-5000
set_galvos_data = {"SetGalvos": [int(curr_dac1), int(curr_dac2)]}
r = requests.post(url=url, json=set_galvos_data)
r.raise_for_status()
query_analog_data = "QueryState"
r = requests.post(url=url, json=query_analog_data)
r.raise_for_status()
curr_adc1=(1.5-r.json()['EchoState']['adc1']*3.3/4096)/10
curr_adc2=(1.5-r.json()['EchoState']['adc2']*3.3/4096)/10
start_time = datetime.datetime.utcnow()
dt=datetime.datetime.utcnow()-start_time
t = int((dt.days * 24 * 60 * 60 + dt.seconds) * 1000 + dt.microseconds / 1000.0)
outdatastr = str(t) + "," + str(r.json()['EchoState']['dac0']) + "," + str(r.json()['EchoState']['dac1'])\
+ "," + str(r.json()['EchoState']['adc1']) + "," + str(r.json()['EchoState']['adc2']) + "\n"
outputfile.write(outdatastr.encode())
print (outdatastr)
while (1):
    del_dac2=(dac2_slope[0][0]*curr_adc1+dac2_intercept[0])-dac2_0
    del_dac1=(dac1_slope[0][0]*curr_adc2+dac1_intercept[0])-dac1_0
    new_dac2=curr_dac2-del_dac2
    new_dac1=curr_dac1-del_dac1
    curr_dac2=new_dac2
    curr_dac1=new_dac1
    #Generate new DACs when the beam is reach the center
    if abs(del_dac2) <0.0001 and abs(del_dac2) <0.0001:
        new_dac1= randint(-5000,  5000)
        new_dac2= randint(-5000,  5000)
    print("DAC1:",int(curr_dac1), "DAC2:",int(curr_dac2))
    #Check if the DACs are OK before commanding the Galvo
    check_OK=input('Checking OK?:y/n')
    if (check_OK=='n'):
        fixed_dac=[int(x) for x in input('Set New Galvo(V):').split()]
        curr_dac1=fixed_dac[0]
        curr_dac2=fixed_dac[1]
    set_galvos_data = {"SetGalvos": [int(curr_dac1), int(curr_dac2)]}
    r = requests.post(url=url, json=set_galvos_data)
    r.raise_for_status()
    query_analog_data = "QueryState"
    r = requests.post(url=url, json=query_analog_data)
    r.raise_for_status()
    curr_adc1=(1.5-r.json()['EchoState']['adc1']*3.3/4096)/10
    curr_adc2=(1.5-r.json()['EchoState']['adc2']*3.3/4096)/10
    dt=datetime.datetime.utcnow()-start_time
    t = int((dt.days * 24 * 60 * 60 + dt.seconds) * 1000 + dt.microseconds / 1000.0)
    outdatastr = str(t) + "," + str(r.json()['EchoState']['dac0']) + "," + str(r.json()['EchoState']['dac1'])\
    + "," + str(r.json()['EchoState']['adc1']) + "," + str(r.json()['EchoState']['adc2']) + "\n"
    outputfile.write(outdatastr.encode())
    print (outdatastr)
   # time.sleep(0.1)


