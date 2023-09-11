git log --format='%aN' | sort -u | while read -r author
do
    echo -n "Author: \"$author\", "
    git log --numstat --author="$author" -- "*.rs" | awk 'NF==3 {plus+=$1} END {printf "Added lines: %s\n", plus; plus=0}'
done
