SRC_CHAIN_ID="target-chain"
DST_CHAIN_ID="malaga-420"
PATH_NAME="ics100_mc"
MAIN1_KEY=""
MAIN2_KEY="track judge phrase loud step uncle play ridge more crawl dragon gospel enjoy ostrich mistake brush have glide arrive favorite vague food invest labor"
BOB_KEY="actress letter whip youth flip sort announce chief traffic side destroy seek parade warrior awake scan panther nominee harsh spawn differ enroll glue become"
MAIN3_KEY="heavy art laptop admit transfer quick loud bag random correct outdoor thing leader pelican taste calm alert ostrich kingdom plunge coil orphan soft explain"

# echo "\nInit Rly config"
# rly config init

# echo "\nAdd sidechain"
# rly chains add --file sidechain.json

# echo "\nAdd source chain"
# rly chains add --file source.json

# echo "\nAdd target chain"
# rly chains add --file target.json

echo "\nAdd malaga chain"
rly chains add --file malaga.json

echo "\nAdd keys"
rly keys restore malaga main3 "$MAIN3_KEY"

echo "\nAdd path"
rly paths new $SRC_CHAIN_ID $DST_CHAIN_ID $PATH_NAME

echo "\nPath tx link"
rly tx link $PATH_NAME

# echo "\bStart path"
# rly start $PATH_NAME