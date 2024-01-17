import asyncio
import time
import cv2
from pymavlink.dialects.v20 import common as mavlink2
from pymavlink import mavutil
import threading

pitch = None
lock = asyncio.Lock()

def thread_mavlink():
    master = mavutil.mavlink_connection("/dev/ttyACM0", 921600)
    master.wait_heartbeat()
    while True:
        msg = master.recv_match(type='ATTITUDE', blocking=True)
        '''lock.acquire()
        pitch = msg.pitch
        lock.release()'''
        print(time.monotonic_ns(), msg.pitch, msg.roll, msg.yaw)


def thread_camera():
    cam = cv2.VideoCapture(0)
    while True:
        result, image = cam.read()
        if result is None:
            break

        lock.acquire().
        p = pitch
        lock.release()
        print(time.monotonic_ns(), p)


t1 = threading.Thread(target=thread_mavlink)
t2 = threading.Thread(target=thread_camera)

t1.start()
t2.start()

t1.join()
t2.join()

'''

'''