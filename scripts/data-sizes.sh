#!/bin/bash

[ -d /var/lib/home-sizes ] || mkdir /var/lib/home-sizes
[ -d /var/lib/prometheus/node-exporter ] || exit

DIFF_FILE="/var/lib/home-sizes/data-diff.txt"
diff_exists_and_non_empty=1

# Check if diff.txt exists and is non-empty
if [ -s "$DIFF_FILE" ]; then
    # Previous difference
    previous_diff=$(cat "$DIFF_FILE")

    # Check if previous_diff is an integer
    if ! [[ "$previous_diff" =~ ^-?[0-9]+$ ]]; then
        echo "$DIFF_FILE does not give an integer. Reset previous diff."
        previous_diff=0
        diff_exists_and_non_empty=0
    fi
else
    # Create a new diff.txt file
    touch "$DIFF_FILE"
    previous_diff=0
    diff_exists_and_non_empty=0
fi

# Run home-sizes-prom command and save the output to a variable
output=$(home-sizes-prom -d 3 -t 30 -p 365 -c /var/lib/home-sizes/data.msgpack /data)
echo "$output" > "/var/lib/prometheus/node-exporter/data_sizes.prom"

# Calculate the new sum using awk on the output variable
new_sum=$(echo "$output" | awk '{ sum += $2 } END { printf "%.0f\n", sum }')

# Calculate current difference
used_space=$(df -B1 --output=used /data | tail -n 1)
current_diff=$((used_space - new_sum))

# Calculate the changed value as a percentage of the total space
total_space=$(df -B1 --output=size /data | tail -n 1)
changed_value=$((current_diff - previous_diff))
changed_value_percentage=$((changed_value * 100 / total_space))
# Take the absolute value of changed_value_percentage
changed_value_percentage=${changed_value_percentage#-}

# Check if the changed value is larger than 1% of the total space
if [[ $diff_exists_and_non_empty -eq 1 && $changed_value_percentage -gt 1 ]]; then
  echo "Changed value ($changed_value B) is larger than 1% of the total space."

  # Remove the data.msgpack file
  rm /var/lib/home-sizes/data.msgpack
  echo "Removed data.msgpack file."

  # Store the new difference in diff.txt
  echo "$current_diff" > "$DIFF_FILE"
  echo "Stored the new difference ($current_diff) in diff.txt."
elif [[ $diff_exists_and_non_empty -eq 0 ]]; then
  echo "No previous diff.txt file found. Creating a new one."

  # Store the new difference in diff.txt
  echo "$current_diff" > "$DIFF_FILE"
  echo "Stored the new difference ($current_diff) in diff.txt."
else
  echo "Changed value ($changed_value B) is not larger than 1% of the total space. No action needed."
fi