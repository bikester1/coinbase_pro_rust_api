#!/bin/sh
COVERAGE=$(grep -A 2 -e "\"heading\">Lines" ./coverage/index.html | grep -o -E ">.*%" | cut -c 2-)
PERCENTAGE=$(echo "$COVERAGE" | rev | cut -c 3- | rev | cut -c -2)

if [ "$PERCENTAGE" -gt 95 ]
then
  COLOR="brightgreen"
elif [ "$PERCENTAGE" -gt 90 ]
then
  COLOR="green"
elif [ "$PERCENTAGE" -gt 80 ]
then
  COLOR="yellow"
else
  COLOR="red"
fi

LINK="https:\/\/img\.shields\.io\/badge\/Coverage-$PERCENTAGE""%25-$COLOR?style=for-the-badge"

LINK2="\/\/\! \!\[coverage\]($LINK)"

sed -i "s/\/\/\! \!\[coverage\].*/$LINK2/" ./src/lib.rs

grep -E "\/\/\!" ./src/lib.rs | cut -c 5- > README.md
