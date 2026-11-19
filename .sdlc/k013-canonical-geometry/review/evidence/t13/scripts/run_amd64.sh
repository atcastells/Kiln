#!/bin/sh
# Runs inside the linux/amd64 container: every case of /jobs/cases.txt through Godot x86_64.
uname -m
/godot/Godot_v4.7.1-stable_linux.x86_64 --headless --version 2>/dev/null | tail -1
start=$(date +%s)
cut -d' ' -f1 /jobs/cases.txt | xargs -P6 -I{} sh -c '/godot/Godot_v4.7.1-stable_linux.x86_64 --headless --path /jobs/{} --script /jobs/{}/generate.gd -- /jobs/{}/job.json /jobs/{}/out.json > /jobs/{}/godot.log 2>&1; echo "{} exit=$? glb=$(test -f /jobs/{}/seed.glb && echo yes || echo no)"'
echo "wall_s=$(( $(date +%s) - start ))"
