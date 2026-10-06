# Login PATH for the reComputer Mini J5012: the sbin directories hold ip and
# friends, and the stock login PATH leaves them out. Appended, never
# prepended, so the original entries keep their order and precedence.
# POSIX sh only (the login shell is BusyBox sh).
for _sbin_dir in /usr/sbin /sbin /usr/local/sbin; do
    case ":$PATH:" in
        *":$_sbin_dir:"*) ;;
        *) PATH="$PATH:$_sbin_dir" ;;
    esac
done
unset _sbin_dir
export PATH
