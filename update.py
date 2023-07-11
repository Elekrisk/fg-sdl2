#!/usr/bin/env python3

import requests
import json
import os
import time
import io
import zipfile
import glob
import shutil

with io.open("config/autoupdate.json") as f:
    config = json.load(f)

current_release = config['current_release']
last_check = config['last_check']
filename = config['filename']

now = time.time()

if now - last_check < 60:
    diff = int(now - last_check)
    print(f"Time since last check is {diff} sec, which is less than the 1 min rate limit")
    exit(0)

config['last_check'] = now
with io.open("config/autoupdate.json", "w") as f:
    f.write(json.dumps(config))

tag_sha_endpoint = "https://api.github.com/repos/elekrisk/fg-sdl2/git/ref/tags/latest"

r = requests.get(tag_sha_endpoint, headers={'Accept': 'application/vnd.github+json'})
response = r.json()

if r.headers['x-ratelimit-remaining'] == '0':
    print("Rate limiting reached; please wait")
    exit(0)

if response['object']['sha'] == current_release:
    print("No update found")
    exit(0)

config['current_release'] = response['object']['sha']

latest_release_endpoint = "https://api.github.com/repos/elekrisk/fg-sdl2/releases/tags/latest"

r = requests.get(latest_release_endpoint, headers={'Accept': 'application/vnd.github+json'})
response = r.json()

if r.headers['x-ratelimit-remaining'] == '0':
    print("Rate limiting reached; please wait")
    exit(0)

assets = response['assets']

asset_download_url = None

for asset in assets:
    if asset['name'] == filename:
        asset_download_url = asset['browser_download_url']
        break

if asset_download_url == None:
    exit(1)

r = requests.get(asset_download_url)

if not os.path.exists("backup"):
    os.mkdir("backup")

x = zipfile.ZipFile(io.BytesIO(r.content))

backup = zipfile.ZipFile(f"backup/backup.zip", "w")
for file in glob.glob("**", recursive=True):
    if os.path.isfile(file) and file.split('/')[0].split('\\')[0] != "backup":
        print(file)
        backup.write(file)

for info in x.filelist:
    if info.filename.split('/')[0] == "config":
        if os.path.exists(info.filename):
            continue
    x.extract(info)

with io.open("config/autoupdate.json", "w") as f:
    f.write(json.dumps(config))

print("Updated")
