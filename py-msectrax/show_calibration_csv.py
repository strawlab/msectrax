import numpy as np
import pandas as pd
import sys
import argparse
from string import Template
import matplotlib
import matplotlib.pyplot as plt

matplotlib.rcParams.update({'font.size': 6})

# minmax = {
#     'dac1': (-500.0, 1.0),
#     'dac2':(-200.0, 1001.0),
#     }

my_template = Template("""data = {
    "SetState": {
        "mode": {"ClosedLoop":"Proportional"},
        "cl_period": 1, # integer range 1 (no delay) to u32 maximum
        "dac1_initial": -11093,
        "dac2_initial": 8853,
        "dac1_angle_func": {
            "adc1_gain": $dac1_adc1_gain,
            "adc2_gain": $dac1_adc2_gain,
            "offset": $dac1_offset,
        },
        "dac2_angle_func": {
            "adc1_gain": $dac2_adc1_gain,
            "adc2_gain": $dac2_adc2_gain,
            "offset": $dac2_offset,
        },
        "dac1_angle_gain": -0.02,
        "dac2_angle_gain": -0.02,
        "dac1_min": -32768,
        "dac1_max": 32767,
        "dac2_min": -32768,
        "dac2_max": 32767,
    }
}
""")

if 1:
    parser = argparse.ArgumentParser()
    parser.add_argument("csv_filename")
    parser.add_argument("--no-cal", help="do not perform calibration", action="store_true")
    parser.add_argument("--no-plot", help="do draw plots", action="store_true")
    args = parser.parse_args()

    do_cal = not args.no_cal
    do_plot = not args.no_plot

    fname = args.csv_filename
    df=pd.read_csv(fname, comment='#')
    df_full = df.copy()
    # auto define central region
    adc1_temp=df['adc1']
    max_idx = [index for index,value in enumerate(adc1_temp) if value==max(adc1_temp)]
    min_idx = [index for index,value in enumerate(adc1_temp) if value==min(adc1_temp)]
    max_adc1=df['dac2'][max_idx[0]]
    min_adc1=df['dac2'][min_idx[0]]
    if max_adc1<min_adc1:
        max_dac2=min_adc1
        min_dac2=max_adc1
    else:
        max_dac2=max_adc1
        min_dac2=min_adc1

    adc2_temp=df['adc2']
    max_idx = [index for index,value in enumerate(adc2_temp) if value==max(adc2_temp)]
    min_idx = [index for index,value in enumerate(adc2_temp) if value==min(adc2_temp)]
    max_adc2=df['dac1'][max_idx[0]]
    min_adc2=df['dac1'][min_idx[0]]  
    if max_adc2<min_adc2:
        max_dac1=min_adc2
        min_dac1=max_adc2
    else:
        max_dac1=max_adc2
        min_dac1=min_adc2

    minmax = {
    'dac1': (min_dac1, max_dac1),
    'dac2':(min_dac2, max_dac2),
    }
    print ('# Calib file:', fname)
    print ('# Central region:', minmax) 

    if 1:
        for dac_name in minmax:
            this_min, this_max = minmax[dac_name]
            df = df[(df[dac_name]>this_min) & (df[dac_name]<this_max)]

    # Perform a linear least squares fit to find A for y = Ap where p is the
    # parameter vector to be fit, A is a matrix built of the ADC values (and ones)
    # and y is the angle value vector (in DAC units).
    # Thus, angle = [adc1, adc2, 1.0] . [p[0], p[1], p[2]]

    if do_cal:
        A = np.vstack( (df['adc1'].values, df['adc2'].values, np.ones_like(df['adc1'].values) ) ).T
        p_dac1_result = np.linalg.lstsq(A, df['dac1'].values, rcond=None)
        p_dac2_result = np.linalg.lstsq(A, df['dac2'].values, rcond=None)

        p_dac1 = p_dac1_result[0]
        p_dac2 = p_dac2_result[0]

        angle1_str = '# Angle1 (DAC1) = {:3g}*ADC1 + {:3g}*ADC2 + {:3g}'.format(*p_dac1)
        angle2_str = '# Angle2 (DAC2) = {:3g}*ADC1 + {:3g}*ADC2 + {:3g}'.format(*p_dac2)
        print(angle1_str)
        print(angle2_str)
        print()
        formatted = my_template.substitute(
            dac1_adc1_gain=p_dac1[0],
            dac1_adc2_gain=p_dac1[1],
            dac1_offset=p_dac1[2],

            dac2_adc1_gain=p_dac2[0],
            dac2_adc2_gain=p_dac2[1],
            dac2_offset=p_dac2[2],
        )
        print(formatted)

        df['dac1_fit'] = np.dot(A,p_dac1)
        df['dac2_fit'] = np.dot(A,p_dac2)

    if do_plot:
        fig, axes = plt.subplots(nrows=2, ncols=2, sharex=True, sharey=True)
        # DAC x axis, ADC y axis
        for j, adc_name in enumerate(('adc1', 'adc2')):

            for i, (x_dac_name, group_dac_name) in enumerate([('dac1', 'dac2'),
                ('dac2', 'dac1')]):

                ax = axes[j,i]

                for group_dac_value, gdf in df_full.groupby(group_dac_name):
                    ax.plot(gdf[x_dac_name], gdf[adc_name], label='%s = %s'%(group_dac_name, group_dac_value))

                this_min, this_max = minmax[x_dac_name]
                ax.axvline(  this_min )
                ax.axvline(  this_max )

                ax.set_ylabel(adc_name)
                ax.set_xlabel(x_dac_name)
                ax.legend(loc='upper right', prop={'size': 4})
                ax.xaxis.set_major_locator(plt.MaxNLocator(5))
                ax.yaxis.set_major_locator(plt.MaxNLocator(5))

        fig.text(0.5, 1.0, fname, horizontalalignment='center', verticalalignment='top')
        out_fname = fname + '.calibration.png'
        print('saving %s'%out_fname)
        plt.savefig(out_fname, dpi=300)

    if do_plot:
        fig, axes = plt.subplots(nrows=2, ncols=2, sharex=True, sharey=True)
        # ADC x axis, DAC y axis
        for j, adc_name in enumerate(('adc1', 'adc2')):

            for i, (y_dac_name, group_dac_name) in enumerate([('dac1', 'dac2'),
                ('dac2', 'dac1')]):

                ax = axes[j,i]

                for group_dac_value, gdf in df.groupby(group_dac_name):
                    ax.plot(gdf[adc_name], gdf[y_dac_name], 'x', label='%s = %s'%(group_dac_name, group_dac_value))
                    if do_cal:
                        ax.plot(gdf[adc_name], gdf[y_dac_name+'_fit'], '-')

                ax.set_xlabel(adc_name)
                ax.set_ylabel('Angle (%s units)'%(y_dac_name,))
                ax.legend(loc='upper right', prop={'size': 4})
                ax.xaxis.set_major_locator(plt.MaxNLocator(5))
                ax.yaxis.set_major_locator(plt.MaxNLocator(5))

        if do_cal:
            fig.text(0.5, 1.0, '\n'.join([fname, angle1_str, angle2_str]),
                horizontalalignment='center', verticalalignment='top')
        out_fname = fname + '.fit-cal.png'
        print('saving %s'%out_fname)
        plt.savefig(out_fname, dpi=300)

    if do_plot:
        plt.show()
