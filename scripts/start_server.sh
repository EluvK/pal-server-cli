#!/bin/bash

ps -ef | grep PalServer | grep -v grep | awk -F ' ' '{print $2}' | xargs kill -9

sleep 2

cd ~/Steam/steamapps/common/PalServer

nohup ./PalServer.sh -useperfthreads -NoAsyncLoadingThread -UseMultithreadForDS -players=8 &

if [ $? -eq 0 ]; then
    sleep 2
    echo "Server started"
else
    echo "failed"
    exist -1
fi
