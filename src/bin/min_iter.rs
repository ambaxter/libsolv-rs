extern crate libc;
extern crate libsolv;
extern crate libsolv_sys;
extern crate libsolvext_sys;

use std::ffi::{CStr, CString};
use std::ptr;
use std::mem;
use std::slice;

use libsolv_sys::{_Pool, Pool, Dataiterator};
use libsolv_sys::{pool_create, pool_setdebuglevel};
use libsolv_sys::pool_free;
use libsolv_sys::pool_setarch;
use libsolvext_sys::solv_xfopen;
use libsolv_sys::Repo;
use libsolv_sys::_Chksum;
use libsolv_sys::repo_create;
use libsolv_sys::repo_free;
use libsolvext_sys::repo_add_repomdxml;
use libsolv_sys::{SEARCH_STRING, SOLVID_META};
use libsolv_sys::{solv_knownid, Id};
use libsolv_sys::dataiterator_free;
use libsolv_sys::dataiterator_init;
use libsolv_sys::dataiterator_prepend_keyname;
use libsolv_sys::dataiterator_step;
use libsolv_sys::{dataiterator_init_clone, dataiterator_strdup};
use libsolv_sys::dataiterator_setpos_parent;
use libsolv_sys::{SOLVID_POS, pool_lookup_str, pool_lookup_bin_checksum, solv_chksum_create_from_bin};
use libsolv_sys::solv_chksum_free;

unsafe fn find(pool: *mut Pool, repo: *mut Repo, what: &CStr) -> (Option<CString>, Option<*mut _Chksum>) {
    let mut lookup_cstr = None;
    let mut lookup_chksum = None;

    let mut di = {
        let mut di = mem::zeroed();
        dataiterator_init(&mut di, pool, repo,
                          SOLVID_META as Id, solv_knownid::REPOSITORY_REPOMD_TYPE as Id, what.as_ptr(), SEARCH_STRING as Id);
        dataiterator_prepend_keyname(&mut di, solv_knownid::REPOSITORY_REPOMD as Id);
        di
    };

    while dataiterator_step(&mut di) != 0 {
        println!("loop!");
        let mut ndi = {
            let mut ndi = mem::zeroed();
            dataiterator_init_clone(&mut ndi, &mut di);
            dataiterator_strdup(&mut ndi);
            ndi
        };

        let pos = {
            let _pool: &mut _Pool =  &mut *ndi.pool;
            let old_pos = _pool.pos;
            dataiterator_setpos_parent(&mut ndi);
            let pos = _pool.pos;
            _pool.pos = old_pos;

            println!("pos: {:?}", &pos);
            pos
        };
        lookup_cstr = {
            let repo: &mut Repo = &mut *pos.repo;
            let _pool: &mut _Pool = &mut *repo.pool;
            let old_pos = _pool.pos;
            _pool.pos = pos;
            let cstr = pool_lookup_str(_pool, SOLVID_POS, solv_knownid::REPOSITORY_REPOMD_LOCATION as Id);
            _pool.pos = old_pos;
            if cstr.is_null() {
                None
            } else {
                    let len = libc::strlen(cstr);
                    let slice = slice::from_raw_parts(cstr as *const libc::c_uchar, len as usize);
                    CString::new(slice).ok()
            }
        };
        println!("cstr: {:?}", lookup_cstr);

        lookup_chksum = {
            let repo: &mut Repo = &mut *pos.repo;
            let _pool: &mut _Pool = &mut *repo.pool;
            let old_pos = _pool.pos;
            _pool.pos = pos;
            let mut type_id = 0;
            let b = pool_lookup_bin_checksum(_pool, SOLVID_POS, solv_knownid::REPOSITORY_REPOMD_CHECKSUM as Id, &mut type_id);
            _pool.pos = old_pos;
            let _c = solv_chksum_create_from_bin(type_id, b);
            if _c.is_null() {
                None
            } else {
                Some(_c)
            }
        };
        println!("chksum: {:?}", lookup_chksum);

        dataiterator_free(&mut ndi);

        if lookup_cstr.is_some() {
            break;
        }

    }
    dataiterator_free(&mut di);

    (lookup_cstr, lookup_chksum)
}

unsafe fn load_repo(pool: *mut _Pool, path: &CStr) {
    let readonly = CString::new("r").unwrap();
    let repomd_fp = solv_xfopen(path.as_ptr(), readonly.as_ptr());
    assert!(!repomd_fp.is_null());

    let repo_name = CString::new("min_unsfe").unwrap();
    let repo = repo_create(pool, repo_name.as_ptr());

    repo_add_repomdxml(repo, repomd_fp, 0);

    libc::fclose(repomd_fp);

    let what = CString::new("primary").unwrap();

    let (o_name, o_chksum) = find(pool, repo, &what);
    let primary_name = o_name.expect("Expected name");
    let primary_chksum = o_chksum.expect("Expected checksum");

    solv_chksum_free(primary_chksum, ptr::null_mut());
    repo_free(repo, 0);
}

fn main() {
    // Create the pool
    let pool = unsafe{pool_create()};
    unsafe{pool_setdebuglevel(pool, 2)};
    // Set the pool arch
    let arch = CString::new("x86_64").unwrap();
    let path = CString::new("files/repomd.xml").unwrap();

    unsafe{
        pool_setarch(pool, arch.as_ptr());
        load_repo(pool, &path);
    };

    unsafe{pool_free(pool)};
}