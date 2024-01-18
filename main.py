import time
import cv2
from pymavlink import mavutil
import threading
import queue
import sqlite3

pitch = None
lock = threading.Lock()

queue = queue.Queue()


def thread_mavlink():
    master = mavutil.mavlink_connection("/dev/ttyACM0", 921600)
    master.wait_heartbeat()
    while True:
        msg = master.recv_match(type='ATTITUDE', blocking=True)
        lock.acquire(True)
        global pitch
        pitch = (time.monotonic(), msg.pitch)
        lock.release()


def thread_camera():
    cam = cv2.VideoCapture(0)
    while True:
        result, image = cam.read()
        if result is None:
            break

        lock.acquire(True)
        global pitch
        p = pitch
        # print(time.monotonic_ns(), p)
        lock.release()
        queue.put((p[0], p[1], time.monotonic(), image))


def thread_db():
    db = sqlite3.connect("db.sqlite")
    cursor = db.cursor()
    cursor.execute("""CREATE TABLE "main" (
        "index"	INTEGER NOT NULL UNIQUE,
        "time_p"	INTEGER NOT NULL,
        "pitch"	REAL,
        "time_i"	INTEGER,
        "img"	BLOB,
        PRIMARY KEY("index" AUTOINCREMENT)
    );""")
    db.commit()
    while True:
        item = queue.get()
        print(item[0])
        im = cv2.imencode('.jpg', item[3])[1]
        cursor.execute(""" INSERT INTO main (time_p, pitch, time_i, img) VALUES (?, ?, ?, ?)""",
                       (item[0], item[1], item[2], im))
        db.commit()


t1 = threading.Thread(target=thread_mavlink)
t2 = threading.Thread(target=thread_camera)
t3 = threading.Thread(target=thread_db)

t1.start()
t2.start()
t3.start()

t1.join()
t2.join()
t3.join()
