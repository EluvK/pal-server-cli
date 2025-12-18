#!/bin/bash

set -x

sudo add-apt-repository multiverse -y; sudo dpkg --add-architecture i386; sudo apt update
sudo apt install lib32gcc-s1 -y
# sudo apt install steamcmd -y

mkdir -p ~/Steam && cd ~/Steam && curl -sqL "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz" | tar zxvf -

mkdir -p ~/.steam/sdk64/
max_retries=3
retries=0

while [ $retries -lt $max_retries ]
do
  ./steamcmd.sh +login anonymous +app_update 1007 +quit
  if [ $? -eq 0 ]; then
    echo "success"
    break
  else
    echo "failed, retrying..."
    retries=$((retries+1))
  fi
done
if [ $retries -eq $max_retries ]; then
  echo "Max retries exceeded, exiting..."
  exit -1
fi

cp ~/Steam/steamapps/common/Steamworks\ SDK\ Redist/linux64/steamclient.so ~/.steam/sdk64/

retries=0
while [ $retries -lt $max_retries ]
do
  ./steamcmd.sh +login anonymous +app_update 2394010 validate +quit
  if [ $? -eq 0 ]; then
    echo "success"
    break
  else
    echo "failed, retrying..."
    retries=$((retries+1))
  fi
done
if [ $retries -eq $max_retries ]; then
  echo "Max retries exceeded, exiting..."
  exit -1
fi

echo "Server installed"
