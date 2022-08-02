//! Helper functions related to Tock processes by OTA_app. 
use core::cmp;

use crate::debug;
use crate::config;
use crate::kernel::Kernel;
use crate::platform::chip::Chip;
use crate::process::Process;
use crate::process_policies::ProcessFaultPolicy;
use crate::process_standard::ProcessStandard;
use crate::process_utilities::ProcessLoadError;
use crate::capabilities::MemoryAllocationCapability;
use crate::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use crate::syscall_driver::{CommandReturn, SyscallDriver};
use crate::process::ProcessId;
use crate::ErrorCode;

pub const DRIVER_NUM: usize = 0x10001;

mod ro_allow {
    /// Ids for read-only allow buffers ('_' means no use)
    pub(crate) const _WRITE: usize = 0;
    /// The number of allow buffers the kernel stores for this grant
    pub(crate) const COUNT: usize = 1;
}

mod rw_allow {
    /// Ids for read-write allow buffers ('_' means no use)
    pub(crate) const _READ: usize = 0;
    /// The number of allow buffers the kernel stores for this grant
    pub(crate) const COUNT: usize = 1;
}

/// Variable that is stored in OTA_app grant region to support dynamic app load
#[derive(Default)]
struct ProcLoaderData{
    //Index points the position where the entry point of a new app is written into PROCESS global array 
    index: usize,
    // App size requested by ota app
    appsize_requested_by_ota_app: usize,
    // dynamic_flash_start_addr points the start address that a new app will be loaded
    dynamic_flash_start_addr: usize,
    // dynamic_unsued_sram_start_addr points the start address that a new app will use
    dynamic_unsued_sram_start_addr: usize,
}

pub struct ProcessLoader <C:'static + Chip>{
    kernel: &'static Kernel,
    chip: &'static C, 
    fault_policy: &'static dyn ProcessFaultPolicy,
    ptr_process: *mut Option<&'static (dyn Process + 'static)>,
    ptr_process_region_start_address: *mut usize,
    ptr_process_region_size: *mut usize,
    supported_process_num: usize,
    start_app: usize,
    end_app: usize,
    end_appmem: usize,
    dynamic_unused_ram_start_addr_init_val: &'static usize,
    index_init_val: &'static usize,
    data: Grant<
        ProcLoaderData,
        UpcallCount<2>,
        AllowRoCount<{ ro_allow::COUNT }>,
        AllowRwCount<{ rw_allow::COUNT }>,
    >,
}

impl <C:'static + Chip> ProcessLoader <C> {
    pub fn init(
        kernel: &'static Kernel,
        chip: &'static C,
        fault_policy: &'static dyn ProcessFaultPolicy,
        memcapability: &dyn MemoryAllocationCapability,
        ptr_process: *mut Option<&'static (dyn Process + 'static)>,
        ptr_process_region_start_address: *mut usize,
        ptr_process_region_size: *mut usize,
        supported_process_num: usize,
        start_app: usize,
        end_app: usize,
        end_appmem: usize,
        dynamic_unused_ram_start_addr_init_val: &'static usize,
        index_init_val: &'static usize,
    ) -> ProcessLoader <C> {
        ProcessLoader {
            kernel: kernel,
            chip: chip, 
            fault_policy: fault_policy,
            ptr_process: ptr_process,
            ptr_process_region_start_address: ptr_process_region_start_address,
            ptr_process_region_size: ptr_process_region_size,
            supported_process_num: supported_process_num,
            start_app: start_app,
            end_app: end_app,
            end_appmem: end_appmem,
            dynamic_unused_ram_start_addr_init_val: dynamic_unused_ram_start_addr_init_val,
            index_init_val: index_init_val,
            data: kernel.create_grant(DRIVER_NUM, memcapability),
        }
    }

    // This function is implemented based on load_processes_advanced
    // the purpose is to load an application flashed from OTA_app into PROCESS global array
    fn load_processes_advanced_air(
        &self,
        proc_data: &mut ProcLoaderData,
    ) -> Result<(usize, Option<&'static dyn Process>), ProcessLoadError> { 

        let appstart = proc_data.dynamic_flash_start_addr as *const u8;
        let appsramstart = proc_data.dynamic_unsued_sram_start_addr as *mut u8;
        
        let mut sram_end_addresses = 0;
        let mut process_copy: Option<&'static dyn Process> = None;

        //Todo: self.eapps has to be replaced by the end address of the flahsed app? (can reduce the ram usage)
        let remaining_flash =  unsafe {
            core::slice::from_raw_parts(
            appstart,
            self.end_app - appstart as usize,
        )};

        let remaining_memory = unsafe {
            core::slice::from_raw_parts_mut(
            appsramstart,
            self.end_appmem - appsramstart as usize,
        )};

        if proc_data.index < self.supported_process_num 
        {        
            // Get the first eight bytes of flash to check if there is another app.
             let test_header_slice = match remaining_flash.get(0..8) {
                Some(s) => s,
                None => {
                    // Not enough flash to test for another app. This just means
                    // we are at the end of flash, and there are no more apps to
                    // load. => This case is error in loading app by ota_app, because it means that there is no valid tbf header!
                    return Err(ProcessLoadError::InternalError);
                }
            };
    
            // Pass the first eight bytes to tbfheader to parse out the length of
            // the tbf header and app. We then use those values to see if we have
            // enough flash remaining to parse the remainder of the header.
            let (version, header_length, entry_length) = match tock_tbf::parse::parse_tbf_header_lengths(
                test_header_slice
                    .try_into()
                    .or(Err(ProcessLoadError::InternalError))?,
            ) {
                Ok((v, hl, el)) => (v, hl, el),
                Err(tock_tbf::types::InitialTbfParseError::InvalidHeader(_entry_length)) => {
                    // If we could not parse the header, then we want to skip over
                    // this app and look for the next one. => This case is error in loading app by ota_app
                    return Err(ProcessLoadError::InternalError);
                }
                Err(tock_tbf::types::InitialTbfParseError::UnableToParse) => {
                    // Since Tock apps use a linked list, it is very possible the
                    // header we started to parse is intentionally invalid to signal
                    // the end of apps. This is ok and just means we have finished
                    // loading apps. => This case is error in loading app by ota_app
                    return Err(ProcessLoadError::InternalError);
                }
            };

            // Now we can get a slice which only encompasses the length of flash
            // described by this tbf header.  We will either parse this as an actual
            // app, or skip over this region.
            let entry_flash = remaining_flash
                .get(0..entry_length as usize)
                .ok_or(ProcessLoadError::NotEnoughFlash)?;

            // Need to reassign remaining_memory in every iteration so the compiler
            // knows it will not be re-borrowed.
            if header_length > 0 
            {
                // If we found an actual app header, try to create a `Process`
                // object. We also need to shrink the amount of remaining memory
                // based on whatever is assigned to the new process if one is
                // created.
        
                // Try to create a process object from that app slice. If we don't
                // get a process and we didn't get a loading error (aka we got to
                // this point), then the app is a disabled process or just padding.
                let (process_option, _unused_memory) = unsafe {
                    ProcessStandard::create(
                        self.kernel,
                        self.chip,
                        entry_flash,
                        header_length as usize,
                        version,
                        remaining_memory,
                        self.fault_policy,
                        true,
                        proc_data.index,
                    )?
                };
                process_option.map(|process| {
                    if config::CONFIG.debug_load_processes {
                        let addresses = process.get_addresses();
                            debug!(
                            "Loaded process[{}] from flash={:#010X}-{:#010X} into sram={:#010X}-{:#010X} = {:?}",
                            proc_data.index,
                            entry_flash.as_ptr() as usize,
                            entry_flash.as_ptr() as usize + entry_flash.len() - 1,
                            addresses.sram_start,
                            addresses.sram_end - 1,
                            process.get_process_name()
                        );
                    }
                    
                    //we return sram_end_addresses
                    let addresses = process.get_addresses();
                    sram_end_addresses = addresses.sram_end;

                    //we return process_copy
                    process_copy = Some(process);
                });
            }
            else {
                //header length 0 means invalid header
                return Err(ProcessLoadError::InternalError);
            }
        }

        Ok((sram_end_addresses, process_copy))
    }

    // In order to match the result value of command
    fn load_processes_air(
        &self,
        proc_data: &mut ProcLoaderData,
    ) -> Result<(), ErrorCode> {
        
        let res = self.load_processes_advanced_air(proc_data);

        // Without alignment error, we only store the entry point, the start address, and the size of the flashed application
        match res{
            Ok((sram_end, process_copy)) => {
                // This variable will be used, when loading the another app at next load work by ota app
                // This is necessary to prevent the access violation of sram memory whilch are already used by kernel and other apps.
                proc_data.dynamic_unsued_sram_start_addr = sram_end;
                
                // Store the entry point, the start address, and the size of the flashed application into PROCESS global array
                // Although I used unsafe keyword, I think it's okay, becasue we pass the exact pointer of PROCESS global array
                unsafe {
                    *self.ptr_process.offset(proc_data.index.try_into().unwrap()) = process_copy;

                    // We also save process region information to check the validity of 'proc_data.dynamic_flash_start_addr' in future load work
                    *self.ptr_process_region_start_address.offset(proc_data.index.try_into().unwrap()) = proc_data.dynamic_flash_start_addr;
                    *self.ptr_process_region_size.offset(proc_data.index.try_into().unwrap()) = proc_data.appsize_requested_by_ota_app;
                }

                // We increase the index for next load work by OTA app
                proc_data.index += 1;

                return Ok(());
            }
            Err(_e) => {
                // If there is an error caused by misalignment,
                // 'proc_data.dynamic_unsued_sram_start_addr' and 'proc_data.index' will hold current unused sram start address
                return Err(ErrorCode::FAIL);
            }
        }
    }
    
    // This function is implemented based on load_processes_advanced
    // the purpose is to parse the dynamically changing start address of flash memory satisfying MPU rules
    fn find_dynamic_start_address_of_writable_flash_advanced(
        &self,
        proc_data: &mut ProcLoaderData,
        start_app: usize,
    ) -> Result<(), ProcessLoadError> {

        let mut app_start_address: usize = start_app;

        while app_start_address < self.end_app
        {
            //We only need tbf header information to get the size of app which is already loaded
            let header_info = unsafe {
                core::slice::from_raw_parts(
                    app_start_address as *const u8,
                    8,
                )
            };

            let test_header_slice = match header_info.get(0..8) {
                Some(s) => s,
                None => {
                    // Not enough flash to test for another app. This just means
                    // We are at the end of flash (0x80000). => This case is Error!
                    // But we can't reach out to here in this while statement!
                    return Err(ProcessLoadError::NotEnoughFlash);
                }
            };
            
            let (_version, _header_length, entry_length) = match tock_tbf::parse::parse_tbf_header_lengths(
                test_header_slice
                    .try_into()
                    .or(Err(ProcessLoadError::InternalError))?,
            ) {
                Ok((v, hl, el)) => (v, hl, el),
                Err(tock_tbf::types::InitialTbfParseError::InvalidHeader(entry_length)) => {
                    // If we could not parse the header, then we want to skip over
                    // this app and look for the next one.
                    (0, 0, entry_length)
                }
                Err(tock_tbf::types::InitialTbfParseError::UnableToParse) => {
                    // Since Tock apps use a linked list, it is very possible the
                    // header we started to parse is intentionally invalid to signal
                    // the end of apps. This is ok and just means we have finished
                    // loading apps. => This case points the writable 'app_start_address' satisfying MPU rules for an new app

                    // Before return Ok, we check whether or not 'proc_data.dynamic_flash_start_addr' + 'appsize_requested_by_ota_app' invades other regions which is already occupied by other apps
                    // Since app_start_address jumps based on power of 2, there is no overlap region case.
                    // However, we check overlap region for fail-safety!
                    // If there is the overlap region, we try to find another 'app_start_address' satisfying MPU rules recursively
                    let address_validity_check = self.check_overlap_region(proc_data);

                    match address_validity_check{
                        Ok(()) => {
                            return Ok(());
                        }
                        Err((new_start_addr, _e)) => {
                            // We try to parse again from the new start address point!
                            app_start_address = new_start_addr;

                            let new_header_slice =  unsafe {
                                core::slice::from_raw_parts(
                                app_start_address as *const u8,
                                8,
                            )};
                        
                            let new_entry_length = usize::from_le_bytes([new_header_slice[4], new_header_slice[5], new_header_slice[6], new_header_slice[7]]);
                            
                            // entry_length is replaced by new_entry_length
                            (0 as u16, 0 as u16, new_entry_length as u32)
                        }
                    }
                }
            };

            // Jump to the maximum length based on power of 2
            // If 'tockloader' offers flashing app bundles with MPU subregion rules (e.g., 16k+32k consecutive), we need more logic!
            // We have to check whether or not there is subregion, and jump to the start address of 32k (Todo!)
            app_start_address += cmp::max(proc_data.appsize_requested_by_ota_app, entry_length.try_into().unwrap());
            proc_data.dynamic_flash_start_addr = app_start_address;
        }

        //If we cannot parse a vaild start address satisfying MPU rules, we return Error.
        return Err(ProcessLoadError::NotEnoughFlash);
    }

    // In order to match the result value of command
    fn find_dynamic_start_address_of_writable_flash(
        &self,
        proc_data: &mut ProcLoaderData,
    ) -> Result<(), ErrorCode> {
        
        //First, we check Index validity  
        if proc_data.index >= self.supported_process_num
        {
            return Err(ErrorCode::FAIL); 
        }

        //If there is enough room in PROCESS array, we start to find a start address satisfying MPU rules
        let res = self.find_dynamic_start_address_of_writable_flash_advanced(proc_data, self.start_app);

        match res{
            Ok(()) => {
                return Ok(());
            }
            Err(_e) => {
                return Err(ErrorCode::FAIL);
            }
        }
    }

    // Check validity of 'proc_data.dynamic_flash_start_addr'
    fn check_overlap_region(
        &self,
        proc_data: &mut ProcLoaderData,
    ) -> Result<(), (usize, ProcessLoadError)>{
        
        let mut recal :bool = false;
        let mut index = 0;
        let mut new_process_start_address = proc_data.dynamic_flash_start_addr;
        let new_process_end_address = proc_data.dynamic_flash_start_addr + proc_data.appsize_requested_by_ota_app - 1;

        while index < proc_data.index
        {
            let process_start_address = unsafe { *self.ptr_process_region_start_address.offset(index.try_into().unwrap()) };
            let process_end_address = unsafe{ *self.ptr_process_region_start_address.offset(index.try_into().unwrap()) + *self.ptr_process_region_size.offset(index.try_into().unwrap()) -1 };

            //debug!("process_start_address, process_end_address, {:#010X} {:#010X}", process_start_address, process_end_address);
            //debug!("new_process_start_address, new_process_end_address, {:#010X} {:#010X}", new_process_start_address, new_process_end_address);

            //If Else sequence is intended!
            if new_process_end_address >= process_start_address && new_process_end_address <= process_end_address          
            {
                /* Case 1
                *              _________________          _______________           _________________
                *  ___________|__               |        |              _|_________|__               |
                * |           |  |              |        |             | |         |  |              |
                * |   new app |  |  app2        |   or   |   app1      | | new app |  |  app2        | 
                * |___________|__|              |        |             |_|_________|__|              |
                *             |_________________|        |_______________|         |_________________|
                * 
                * ^...........^                                           ^........^
                * In this case, we discard this region, and we try to find another start address from the start address of app2
                */
                
                return Err((process_start_address, ProcessLoadError::NotEnoughFlash));
            }

            else if new_process_start_address >= process_start_address && new_process_start_address <= process_end_address
            {
                /* Case 2
                *              _________________
                *  ___________|__               |    _______________
                * |           |  |              |   |               |
                * |   app2    |  |  new app     |   |     app3      |         
                * |___________|__|              |   |_______________|
                *             |_________________|
                * 
                *                 ^
                *                 | In this case, the start address of new app is replaced by 'the end address + 1' of app2 . Since we don't know whether or not there are oter apps after pusing new app
                *                   we check whether or not 'the recalibrated new_process_start_address + the length' invades another region occupied by app3
                *                   It means that, if this recalibrated region commits Case 1, we discard this region, and then find another start address from app3
                */

                new_process_start_address = process_end_address + 1;
                proc_data.dynamic_flash_start_addr = new_process_start_address;
                recal = true;
            }

            if recal == true
            {
                //we check from scratch again!
                recal = false;
                index = 0;
            }
            else
            {
                index += 1;
            }
        }

        return Ok(());
    }

    // CRC32_POSIX
    fn cal_crc32_posix(
        &self,
        proc_data: &mut ProcLoaderData,
    ) -> u32 {
        
        let appstart = proc_data.dynamic_flash_start_addr as *const u8;

        //Only parse the header information (8byte)
        let header_slice =  unsafe {
            core::slice::from_raw_parts(
            appstart,
            8,
        )};
       
        let entry_length = usize::from_le_bytes([header_slice[4], header_slice[5], header_slice[6], header_slice[7]]);
        
        let data =  unsafe {
            core::slice::from_raw_parts(
            appstart,
            entry_length,
        )};

        let mut crc32_instance = tickv::crc32::Crc32::new();
        crc32_instance.update(data);
        
        let crc32_rst = crc32_instance.finalise();

        return crc32_rst;
    }
}

impl <C:'static + Chip> SyscallDriver for ProcessLoader <C> {
    /// ### `command_num`
    ///
    /// - `0`: Driver check, always returns Ok(())
    /// - `1`: Perform loading an process flashed from OTA_app and write the entry point of the process into PROCESS global array
    /// - `2`: Perform finding dynamically changing start address of writable flash memory based on MPU rules
    /// - `3`: Return the dynamically changing start address after commnad 2 in order to control offset of flash region from 'ota_app'
    /// - `4`: Initialize 'proc_data.dynamic_unsued_sram_start_addr' and 'proc_data.index' with sram_end_address and index returned from load_processes_advanced respectively
    ///        This initial values come from the result value of 'kernel::process::load_processes' at main.rs (This commnad is only executed one time at OTA_app init stage)
    ///        This inital value is copied to internal grant variables, and this grant variables is used in 'fn load_processes_advanced_air' and updated after loading an application
    ///        Note that we don't have to interrupt the sram region already used by kernel and other apps
    /// - `5`: Calculate CRC32-POXIS of the flashed app region and return the result value
    /// - `6`: Return an index that is used to store the entry point of an app flashed into PROCESS global array
    ///        With this index, we prevent the kernel from loading 4 more than applications
    /// - `7`: Return the start address of flash memory allocated to apps (i.e., 0x40000 in case of this platform)
    
    fn command(
        &self,
        command_num: usize,
        arg1: usize,
        _unused2: usize,
        appid: ProcessId,
    ) -> CommandReturn {
        match command_num {
            0 => CommandReturn::success(),
            
            1 =>
            /* perform load process work */
            {
                let res = self.data.enter(appid, |proc_data, _| {
                    self.load_processes_air(proc_data)
                })
                .map_err(ErrorCode::from);
        
                match res {
                    Ok(Ok(())) => CommandReturn::success(),
                    Ok(Err(e)) => CommandReturn::failure(e),
                    Err(e) => CommandReturn::failure(e),
                }
            }

            2 =>
            /* find dynamically changing start address of writable flash memory based on MPU rules */
            {   
                let res = self.data.enter(appid, |proc_data, _| {
                    proc_data.appsize_requested_by_ota_app = arg1;
                    self.find_dynamic_start_address_of_writable_flash(proc_data)
                })
                .map_err(ErrorCode::from);
        
                match res {
                    Ok(Ok(())) => CommandReturn::success(),
                    Ok(Err(e)) => CommandReturn::failure(e),
                    Err(e) => CommandReturn::failure(e),
                }
            }

            3 =>
            /* Return the dynamically changing start address after commnad 2 */
            {
                self.data.enter(appid, |proc_data, _| {
                    CommandReturn::success_u32(proc_data.dynamic_flash_start_addr as u32)
                })
                .unwrap_or(CommandReturn::failure(ErrorCode::FAIL))
            }

            /* Initialize 'proc_data.dynamic_unsued_sram_start_addr' and 'proc_data.index' with sram_end_address and index returned from load_processes_advanced respectively */
            4 =>
            {
                let res = self.data.enter(appid, |proc_data, _| {
                    proc_data.dynamic_unsued_sram_start_addr = *self.dynamic_unused_ram_start_addr_init_val;
                    proc_data.index = *self.index_init_val;
                })
                .map_err(ErrorCode::from);
        
                match res {
                    Ok(()) => CommandReturn::success(),
                    Err(e) => CommandReturn::failure(e),
                }
            }

            5 =>
            /* Calculate CRC32-POXIS of the flashed app region and return the result value */
            {
                self.data.enter(appid, |proc_data, _| {
                    let crc32 = self.cal_crc32_posix(proc_data);
                    CommandReturn::success_u32(crc32 as u32)
                })
                .unwrap_or(CommandReturn::failure(ErrorCode::FAIL))
            }

            6 =>
            /* Return index that is used to store the entry point of an app flashed */
            {
                self.data.enter(appid, |proc_data, _| {
                    CommandReturn::success_u32(proc_data.index as u32)
                })
                .unwrap_or(CommandReturn::failure(ErrorCode::FAIL))
            }

            /* Return the start address of flash memory allocated to apps (i.e., 0x40000 in case of this platform)  */
            7 =>
            {
                CommandReturn::success_u32(self.start_app as u32)
            }

            _ => CommandReturn::failure(ErrorCode::NOSUPPORT),
        }
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), crate::process::Error> {
        self.data.enter(processid, |_, _| {})
    }
}