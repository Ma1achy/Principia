# Windows' counterpart of `scripts/contain.sh` (the owner's CI shape order of 8 Nov 2026, step 1): "run tests and mutants
# inside a Job Object with kill-on-close, a process limit and an 8 GiB per-process memory cap (REQ-VAL-181's equivalent
# of prlimit). Prove it with the same kill test contain.sh check uses." It runs on the self-hosted Windows runner only,
# and nothing is placed there on its account until its check has passed on that runner.
#
#   win_contain.ps1 run <command> <arg>...  With PRIN_CONTAIN=required: runs the command in a Job Object of its own, with
#                                           JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, an active-process limit and
#                                           JOB_OBJECT_LIMIT_PROCESS_MEMORY at 8 GiB, read back before the command
#                                           starts. The command is created in the job (PROC_THREAD_ATTRIBUTE_JOB_LIST),
#                                           so there is no moment it is outside it, and stays suspended until that is
#                                           read back; it gets a hidden console of its own, so a console broadcast from
#                                           it reaches nothing outside. When it ends, whatever it left
#                                           ends with it; if this script is killed, the job's last handle closes and the
#                                           kernel ends the lot. If any of that can't be had, it fails (125): nothing
#                                           runs uncontained where containment is required. Without PRIN_CONTAIN, it
#                                           just runs the command.
#   win_contain.ps1 check                   With PRIN_CONTAIN=required: proves the above on this machine with the
#                                           scenarios of `contain.sh check`, each against a live process outside, and
#                                           fails if any doesn't hold. Without it, says so and passes.
#
# The scenarios, one for one with `contain.sh check` (its probe's exit codes kept where Windows has the same thing):
#   control  contain.sh: `kill -0` sees the outside process.  Here: the leftover lister finds the outside process by its
#            marker, and this process can commit more than the cap, so a refusal inside is the cap's.
#   41       contain.sh: the namespace's init is ours.  Here: the probe is in a job whose limits read back as kill-on-close,
#            the process limit and the 8 GiB cap, at the nesting depth asked for.
#   42       contain.sh: the outside process is unseen.  No Job Object equivalent: Windows has no PID namespace. Measured
#            and reported as a gap, never killed (RQ-751).
#   43       contain.sh: the command's process group is one it can see.  Here: a console of its own (CREATE_NEW_CONSOLE),
#            which its parent isn't on; it then broadcasts CTRL+C to its whole console, Windows' `kill -1`.
#   44       contain.sh: `prlimit --as=8589934592` works inside.  Here: a commit just under the cap is granted and one
#            over it refused.
#   kill -9 -1  contain.sh: the outside process survives it.  Here: it survives the broadcast (43). A kill by pid, which
#            a job doesn't stop, is measured as a gap (42), never sent.
#   46       contain.sh: `setsid sleep` escapes its session, and nothing is left after.  Here: a start breaking away from
#            the job is refused, a detached marked child is left running, and after the run no marked process remains.
#   modes    contain.sh: `run`, then `run contain.sh test` nested.  Here: `run`, then `run` nested in `run` (nextest and
#            cargo make jobs of their own inside ours).
# And two the owner's criterion names that contain.sh's check doesn't:
#   45       the process limit, at a low limit: every start below it is granted and the start at it refused.
#   close    the process tree dies when the job's last handle closes: this script killed mid-run, nothing is left.
$ErrorActionPreference = 'Stop'
$verb, $rest = $args

# REQ-VAL-181's cap, as `prlimit --as` takes it in `scripts/contain.sh`; here it caps what one process may commit.
$MemoryCap = 8589934592
# Applied per R-369: a mutants run's cargo, rustc, nextest and test processes come to low hundreds; a runaway stops here.
$ProcessLimit = 1024
# The low limit the check's scenario 45 runs at.
$CheckProcessLimit = 6
# CreateProcess's refusal of a start breaking away from a job that doesn't allow it (ERROR_ACCESS_DENIED); the process
# limit's refusal is ERROR_NOT_ENOUGH_QUOTA, so only this one proves the job held the start.
$BreakawayRefused = 5
# Each marker is a loopback address in a `ping` that lasts about ten minutes, so the lister finds it by its command line.
$LeakMark = '127.29.97.53'
$OuterMark = '127.59.97.53'

function Fail($message) {
    [Console]::Error.WriteLine("::error::win_contain.ps1: $message")
    # The check's outside process isn't left on the machine.
    if ($script:Outer) { Stop-Process -Id $script:Outer.Id -Force -ErrorAction Ignore }
    exit 125
}

$source = @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

public static class PrinContain {
    public const uint KillOnJobClose = 0x2000;
    public const uint ActiveProcess = 0x8;
    public const uint ProcessMemory = 0x100;
    public const uint Limits = KillOnJobClose | ActiveProcess | ProcessMemory;
    const int AccountingClass = 1;
    const int ExtendedLimitClass = 9;
    const uint CreateSuspended = 0x4;
    // A console of its own, which CREATE_NO_WINDOW doesn't promise ("the console handle for the application is not
    // set"); its window is hidden by STARTF_USESHOWWINDOW with SW_HIDE.
    const uint CreateNewConsole = 0x10;
    const uint ExtendedStartupInfoPresent = 0x80000;
    public const uint CreateBreakawayFromJob = 0x01000000;
    const int UseShowWindow = 0x1;
    const int UseStdHandles = 0x100;
    const short HideWindow = 0;
    // ProcThreadAttributeValue(13, FALSE, TRUE, FALSE): the jobs a process is created in.
    static readonly IntPtr JobListAttribute = new IntPtr(0x2000D);

    [StructLayout(LayoutKind.Sequential)]
    public struct BasicLimit {
        public long PerProcessUserTimeLimit; public long PerJobUserTimeLimit; public uint LimitFlags;
        public UIntPtr MinimumWorkingSetSize; public UIntPtr MaximumWorkingSetSize; public uint ActiveProcessLimit;
        public UIntPtr Affinity; public uint PriorityClass; public uint SchedulingClass;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct IoCounters { public ulong R, W, O, RB, WB, OB; }
    [StructLayout(LayoutKind.Sequential)]
    public struct ExtendedLimit {
        public BasicLimit Basic; public IoCounters Io; public UIntPtr ProcessMemoryLimit; public UIntPtr JobMemoryLimit;
        public UIntPtr PeakProcessMemoryUsed; public UIntPtr PeakJobMemoryUsed;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct Accounting {
        public long TotalUserTime, TotalKernelTime, PeriodUserTime, PeriodKernelTime;
        public uint PageFaults, TotalProcesses, ActiveProcesses, TerminatedProcesses;
    }
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct StartupInfo {
        public int cb; public string Reserved; public string Desktop; public string Title;
        public int X, Y, XSize, YSize, XChars, YChars, Fill, Flags; public short ShowWindow, Reserved2Size;
        public IntPtr Reserved2, StdInput, StdOutput, StdError;
    }
    [StructLayout(LayoutKind.Sequential)]
    struct StartupInfoEx { public StartupInfo Info; public IntPtr AttributeList; }
    [StructLayout(LayoutKind.Sequential)]
    struct ProcessInformation { public IntPtr Process, Thread; public int ProcessId, ThreadId; }
    [StructLayout(LayoutKind.Sequential)]
    struct SecurityAttributes { public int Length; public IntPtr Descriptor; public int Inherit; }

    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr CreateJobObjectW(IntPtr attributes, string name);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool SetInformationJobObject(IntPtr job, int cls, ref ExtendedLimit info, int length);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool QueryInformationJobObject(IntPtr job, int cls, out ExtendedLimit info, int length, IntPtr returned);
    [DllImport("kernel32.dll", SetLastError = true, EntryPoint = "QueryInformationJobObject")]
    static extern bool QueryAccounting(IntPtr job, int cls, out Accounting info, int length, IntPtr returned);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool InitializeProcThreadAttributeList(IntPtr list, int count, int flags, ref IntPtr size);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool UpdateProcThreadAttribute(IntPtr list, uint flags, IntPtr attribute, IntPtr value, IntPtr size,
        IntPtr previous, IntPtr returnSize);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern void DeleteProcThreadAttributeList(IntPtr list);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool TerminateJobObject(IntPtr job, uint code);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool IsProcessInJob(IntPtr process, IntPtr job, out bool result);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern bool CreateProcessW(string application, StringBuilder commandLine, IntPtr processAttributes,
        IntPtr threadAttributes, bool inheritHandles, uint flags, IntPtr environment, string directory,
        ref StartupInfoEx startup, out ProcessInformation info);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool TerminateProcess(IntPtr process, uint code);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool GetExitCodeProcess(IntPtr process, out uint code);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool CloseHandle(IntPtr handle);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr GetStdHandle(int which);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool SetHandleInformation(IntPtr handle, uint mask, uint flags);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr CreateFileW(string name, uint access, uint share, ref SecurityAttributes attributes,
        uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr VirtualAlloc(IntPtr address, UIntPtr size, uint type, uint protect);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool VirtualFree(IntPtr address, UIntPtr size, uint type);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool SetConsoleCtrlHandler(IntPtr handler, bool add);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern bool GenerateConsoleCtrlEvent(uint ctrlEvent, uint group);
    [DllImport("kernel32.dll", SetLastError = true)]
    static extern uint GetConsoleProcessList(uint[] list, uint count);

    static Win32Exception Error(string what) {
        int code = Marshal.GetLastWin32Error();
        return new Win32Exception(code, what + ": " + new Win32Exception(code).Message + " (" + code + ")");
    }

    // Windows' quoting, so that the command sees the arguments it was given (CommandLineToArgvW's rules).
    public static string Quote(string arg) {
        if (arg.Length > 0 && arg.IndexOfAny(new char[] { ' ', '\t', '\n', '\v', '"' }) < 0) return arg;
        StringBuilder sb = new StringBuilder("\"");
        for (int i = 0; ; i++) {
            int slashes = 0;
            while (i < arg.Length && arg[i] == '\\') { i++; slashes++; }
            if (i == arg.Length) { sb.Append('\\', slashes * 2); break; }
            if (arg[i] == '"') { sb.Append('\\', slashes * 2 + 1); sb.Append('"'); }
            else { sb.Append('\\', slashes); sb.Append(arg[i]); }
        }
        sb.Append('"');
        return sb.ToString();
    }

    // The limits of `job`, or, with IntPtr.Zero, of the job the calling process is in.
    public static ExtendedLimit LimitsOf(IntPtr job) {
        ExtendedLimit info;
        if (!QueryInformationJobObject(job, ExtendedLimitClass, out info, Marshal.SizeOf(typeof(ExtendedLimit)), IntPtr.Zero))
            throw Error("QueryInformationJobObject");
        return info;
    }

    // How many processes are alive in `job`, or, with IntPtr.Zero, in the calling process's.
    public static uint ActiveProcesses(IntPtr job) {
        Accounting info;
        if (!QueryAccounting(job, AccountingClass, out info, Marshal.SizeOf(typeof(Accounting)), IntPtr.Zero))
            throw Error("QueryInformationJobObject");
        return info.ActiveProcesses;
    }

    public static bool InAJob() {
        bool result;
        if (!IsProcessInJob(new IntPtr(-1), IntPtr.Zero, out result)) throw Error("IsProcessInJob");
        return result;
    }

    // A job with the three limits, read back: a job that doesn't hold them is never used.
    static IntPtr NewJob(uint processLimit, ulong memoryCap) {
        IntPtr job = CreateJobObjectW(IntPtr.Zero, null);
        if (job == IntPtr.Zero) throw Error("CreateJobObject");
        ExtendedLimit info = new ExtendedLimit();
        info.Basic.LimitFlags = Limits;
        info.Basic.ActiveProcessLimit = processLimit;
        info.ProcessMemoryLimit = new UIntPtr(memoryCap);
        if (!SetInformationJobObject(job, ExtendedLimitClass, ref info, Marshal.SizeOf(typeof(ExtendedLimit)))) {
            Exception e = Error("SetInformationJobObject");
            CloseHandle(job);
            throw e;
        }
        ExtendedLimit back = LimitsOf(job);
        if ((back.Basic.LimitFlags & Limits) != Limits || back.Basic.ActiveProcessLimit != processLimit
            || back.ProcessMemoryLimit.ToUInt64() != memoryCap) {
            CloseHandle(job);
            throw new InvalidOperationException("the job's limits read back as flags 0x" + back.Basic.LimitFlags.ToString("x")
                + ", " + back.Basic.ActiveProcessLimit + " processes, " + back.ProcessMemoryLimit + " bytes");
        }
        return job;
    }

    static IntPtr Inheritable(IntPtr handle) {
        if (handle != IntPtr.Zero && handle != new IntPtr(-1)) SetHandleInformation(handle, 1, 1);
        return handle;
    }

    // NUL, opened for reading as an inheritable handle; the caller closes it.
    static IntPtr Nul() {
        SecurityAttributes sa = new SecurityAttributes();
        sa.Length = Marshal.SizeOf(typeof(SecurityAttributes));
        sa.Inherit = 1;
        IntPtr h = CreateFileW("NUL", 0x80000000, 3, ref sa, 3, 0, IntPtr.Zero);
        if (h == new IntPtr(-1)) throw Error("CreateFile NUL");
        return h;
    }

    // Starts `commandLine` with `flags`; with `std`, hidden, on this process's output and error and NUL's input; with
    // a `job`, created in it, so that it is never outside it.
    static ProcessInformation Start(string commandLine, uint flags, bool std, IntPtr job) {
        StartupInfoEx si = new StartupInfoEx();
        si.Info.cb = Marshal.SizeOf(typeof(StartupInfo));
        IntPtr nul = IntPtr.Zero, list = IntPtr.Zero, jobs = IntPtr.Zero;
        bool listMade = false;
        try {
            if (std) {
                nul = Nul();
                si.Info.Flags = UseStdHandles | UseShowWindow;
                si.Info.ShowWindow = HideWindow;
                si.Info.StdInput = nul;
                si.Info.StdOutput = Inheritable(GetStdHandle(-11));
                si.Info.StdError = Inheritable(GetStdHandle(-12));
            }
            if (job != IntPtr.Zero) {
                IntPtr size = IntPtr.Zero;
                InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref size);
                list = Marshal.AllocHGlobal(size);
                if (!InitializeProcThreadAttributeList(list, 1, 0, ref size)) throw Error("InitializeProcThreadAttributeList");
                listMade = true;
                jobs = Marshal.AllocHGlobal(IntPtr.Size);
                Marshal.WriteIntPtr(jobs, job);
                if (!UpdateProcThreadAttribute(list, 0, JobListAttribute, jobs, new IntPtr(IntPtr.Size), IntPtr.Zero,
                    IntPtr.Zero))
                    throw Error("UpdateProcThreadAttribute PROC_THREAD_ATTRIBUTE_JOB_LIST");
                si.Info.cb = Marshal.SizeOf(typeof(StartupInfoEx));
                si.AttributeList = list;
                flags |= ExtendedStartupInfoPresent;
            }
            ProcessInformation pi;
            if (!CreateProcessW(null, new StringBuilder(commandLine), IntPtr.Zero, IntPtr.Zero, std, flags, IntPtr.Zero,
                null, ref si, out pi))
                throw Error("CreateProcess " + commandLine);
            return pi;
        } finally {
            // The child has its own copy of NUL's handle by now, and its jobs; these, on every path, are freed.
            if (nul != IntPtr.Zero) CloseHandle(nul);
            if (listMade) DeleteProcThreadAttributeList(list);
            if (list != IntPtr.Zero) Marshal.FreeHGlobal(list);
            if (jobs != IntPtr.Zero) Marshal.FreeHGlobal(jobs);
        }
    }

    // Runs `argv` in a new job, created in it and suspended until that is read back, and returns its exit code once it and all it left have ended.
    public static int Run(string[] argv, uint processLimit, ulong memoryCap) {
        StringBuilder line = new StringBuilder();
        foreach (string arg in argv) {
            if (line.Length > 0) line.Append(' ');
            line.Append(Quote(arg));
        }
        IntPtr job = NewJob(processLimit, memoryCap);
        ProcessInformation pi;
        try {
            pi = Start(line.ToString(), CreateSuspended | CreateNewConsole, true, job);
        } catch {
            CloseHandle(job);
            throw;
        }
        bool inJob;
        bool asked = IsProcessInJob(pi.Process, job, out inJob);
        if (!asked || !inJob) {
            Exception e = !asked ? (Exception)Error("IsProcessInJob")
                : new InvalidOperationException("the command was created outside its job");
            TerminateProcess(pi.Process, 125);
            CloseHandle(pi.Thread);
            CloseHandle(pi.Process);
            CloseHandle(job);
            throw e;
        }
        if (ResumeThread(pi.Thread) == 0xFFFFFFFF) {
            Exception e = Error("ResumeThread");
            CloseHandle(pi.Thread);
            CloseHandle(pi.Process);
            // The job's close ends the still-suspended process in it.
            CloseHandle(job);
            throw e;
        }
        CloseHandle(pi.Thread);
        WaitForSingleObject(pi.Process, 0xFFFFFFFF);
        uint code;
        if (!GetExitCodeProcess(pi.Process, out code)) code = 125;
        CloseHandle(pi.Process);
        // What the command left ends with it, as everything in a PID namespace ends with its init.
        TerminateJobObject(job, 1);
        DateTime until = DateTime.UtcNow.AddSeconds(30);
        while (ActiveProcesses(job) > 0) {
            if (DateTime.UtcNow > until) {
                CloseHandle(job);
                throw new InvalidOperationException("processes still in the job 30 s after it was terminated");
            }
            System.Threading.Thread.Sleep(50);
        }
        CloseHandle(job);
        return unchecked((int)code);
    }

    // The probe's: starts `commandLine` on this console with `flags`, returning its pid, or the refusal's error, negated.
    public static int Spawn(string commandLine, uint flags) {
        try {
            ProcessInformation pi = Start(commandLine, flags, false, IntPtr.Zero);
            CloseHandle(pi.Thread);
            CloseHandle(pi.Process);
            return pi.ProcessId;
        } catch (Win32Exception e) {
            return e.NativeErrorCode == 0 ? -1 : -e.NativeErrorCode;
        }
    }

    // Whether this process may open `pid` to terminate it; it is never terminated here.
    public static bool CanTerminate(int pid) {
        IntPtr h = OpenProcess(0x1, false, pid);
        if (h == IntPtr.Zero) return false;
        CloseHandle(h);
        return true;
    }

    public static bool Kill(int pid) {
        IntPtr h = OpenProcess(0x1, false, pid);
        if (h == IntPtr.Zero) return false;
        bool done = TerminateProcess(h, 1);
        CloseHandle(h);
        return done;
    }

    // Commits `size` bytes and frees them again: 0 if granted, else the refusal's error.
    public static int Commit(ulong size) {
        IntPtr p = VirtualAlloc(IntPtr.Zero, new UIntPtr(size), 0x3000, 0x04);
        if (p == IntPtr.Zero) {
            int e = Marshal.GetLastWin32Error();
            return e == 0 ? -1 : e;
        }
        VirtualFree(p, UIntPtr.Zero, 0x8000);
        return 0;
    }

    // The pids on this process's console.
    public static uint[] ConsoleProcesses() {
        uint[] list = new uint[1024];
        uint n = GetConsoleProcessList(list, (uint)list.Length);
        if (n == 0) throw Error("GetConsoleProcessList");
        if (n > list.Length) throw new InvalidOperationException(n + " processes on the console");
        uint[] found = new uint[n];
        Array.Copy(list, found, (int)n);
        return found;
    }

    // CTRL+C to every process on this console, this one ignoring it: Windows' nearest to `kill -1`.
    public static bool Broadcast() {
        SetConsoleCtrlHandler(IntPtr.Zero, true);
        return GenerateConsoleCtrlEvent(0, 0);
    }
}
'@

function Load {
    try {
        Add-Type -TypeDefinition $source -Language CSharp
    } catch {
        Fail "containment required, and the Job Object API can't be had: $(Reason $_)"
    }
}

# The innermost message of an error record or exception.
function Reason($e) {
    $x = if ($e -is [System.Management.Automation.ErrorRecord]) { $e.Exception } else { $e }
    while ($x.InnerException) { $x = $x.InnerException }
    $x.Message
}

function Get-ProcessLimit {
    if (-not $env:PRIN_CONTAIN_PROCESSES) { return $ProcessLimit }
    $n = 0
    if (-not [int]::TryParse($env:PRIN_CONTAIN_PROCESSES, [ref]$n) -or $n -lt 1 -or $n -gt $ProcessLimit) {
        Fail "PRIN_CONTAIN_PROCESSES may only lower the limit, to a whole number from 1 to ${ProcessLimit}: '$env:PRIN_CONTAIN_PROCESSES'"
    }
    $n
}

# Every process but this one whose command line holds `mark`.
function Get-Marked($mark) {
    @(Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($mark) -and $_.ProcessId -ne $PID })
}

function Ping-Line($mark) {
    '"' + (Join-Path $env:SystemRoot 'System32\PING.EXE') + '" -n 600 -w 1000 ' + $mark
}

switch ($verb) {
    'run' {
        if (-not $rest) { Fail 'usage: win_contain.ps1 run <command> <arg>...' }
        if ($env:PRIN_CONTAIN -ne 'required') {
            $command, $commandArgs = $rest
            & $command @commandArgs
            exit $LASTEXITCODE
        }
        if ($env:OS -ne 'Windows_NT') { Fail 'containment required, and this is not Windows' }
        $limit = Get-ProcessLimit
        Load
        # The probe reads this to tell a nested run from one whose inner `run` passed its command through.
        $env:PRIN_CONTAIN_DEPTH = [string](1 + [int]$env:PRIN_CONTAIN_DEPTH)
        try {
            $code = [PrinContain]::Run([string[]]$rest, [uint32]$limit, [uint64]$MemoryCap)
        } catch {
            Fail "containment required, and it failed: $(Reason $_)"
        }
        exit $code
    }
    'probe' {
        # Run by `check`, inside the job: each scenario's result on a line of its own, and its number as the exit code.
        $outer, $limit, $depth = $rest
        Load
        try {
            $l = [PrinContain]::LimitsOf([IntPtr]::Zero)
            $flags = $l.Basic.LimitFlags -band [PrinContain]::Limits
            $at = [int]$env:PRIN_CONTAIN_DEPTH
            "probe: 41: in a job: $([PrinContain]::InAJob()); flags 0x$('{0:x}' -f $l.Basic.LimitFlags); $($l.Basic.ActiveProcessLimit) processes; $($l.ProcessMemoryLimit) bytes; depth $at"
            if (-not [PrinContain]::InAJob() -or $flags -ne [PrinContain]::Limits -or $l.Basic.ActiveProcessLimit -ne [uint32]$limit -or
                $l.ProcessMemoryLimit.ToUInt64() -ne [uint64]$MemoryCap -or $at -ne [int]$depth) {
                exit 41
            }

            $seen = $null -ne (Get-Process -Id ([int]$outer) -ErrorAction Ignore)
            $reach = [PrinContain]::CanTerminate([int]$outer)
            "probe: 42 gap: the outside process $outer is visible: $seen; open to a kill by pid: $reach (a Job Object has no PID namespace)"

            $private = (Get-Process -Id $PID).PrivateMemorySize64
            $under = [uint64]($MemoryCap - $private - 268435456)
            $granted = [PrinContain]::Commit($under)
            $refused = [PrinContain]::Commit([uint64]($MemoryCap + 65536))
            "probe: 44: commit of $under bytes, under the cap: $(if ($granted) { "refused ($granted)" } else { 'granted' }); of $($MemoryCap + 65536), over it: $(if ($refused) { "refused ($refused)" } else { 'granted' })"
            if ($granted -ne 0 -or $refused -eq 0) { exit 44 }

            if ([int]$limit -lt $ProcessLimit) {
                $held = $false
                $base = [PrinContain]::ActiveProcesses([IntPtr]::Zero)
                $started = @()
                for ($i = 1; $i -le [int]$limit + 1; $i++) {
                    $active = [PrinContain]::ActiveProcesses([IntPtr]::Zero)
                    $child = [PrinContain]::Spawn((Ping-Line $LeakMark), 0)
                    if ($child -gt 0) { $started += $child }
                    if ($active -lt [int]$limit) {
                        if ($child -le 0) { "probe: 45: start $i refused at $active of $limit active ($(-$child))"; exit 45 }
                    } else {
                        if ($child -gt 0) { [PrinContain]::Kill($child) | Out-Null; "probe: 45: start $i granted at $active of $limit active"; exit 45 }
                        "probe: 45: every start below $limit active granted; start $i, at $active, refused ($(-$child))"
                        $held = $true
                        break
                    }
                }
                if (-not $held) { "probe: 45: the limit of $limit was never reached"; exit 45 }
                # Its children give their slots back before 43 and 46, ended by pid and waited for: 43's broadcast
                # isn't waited for, and a slot must be free before 46 starts.
                foreach ($c in $started) { [PrinContain]::Kill($c) | Out-Null }
                $until = (Get-Date).AddSeconds(30)
                while (($active = [PrinContain]::ActiveProcesses([IntPtr]::Zero)) -gt $base) {
                    if ((Get-Date) -gt $until) { "probe: 45: $active of $limit still active 30 s after its $($started.Count) children were ended, $base before"; exit 45 }
                    Start-Sleep -Milliseconds 50
                }
                "probe: 45: its $($started.Count) children ended; $active of $limit active, as before it"
            }

            $parent = [int](Get-CimInstance Win32_Process -Filter "ProcessId = $PID").ParentProcessId
            $console = [PrinContain]::ConsoleProcesses()
            "probe: 43: on its console: $($console -join ', '); its parent: $parent"
            if ($console -contains [uint32]$parent) { exit 43 }
            if (-not [PrinContain]::Broadcast()) { 'probe: 43: the CTRL+C broadcast could not be sent'; exit 43 }
            'probe: 43: CTRL+C broadcast to the whole console'

            # A slot is free, so a refusal can only be the job's breakaway rule, not the process limit.
            $active = [PrinContain]::ActiveProcesses([IntPtr]::Zero)
            if ($active -ge [int]$limit) { "probe: 46: no slot free for the breakaway start: $active of $limit active"; exit 46 }
            $broke = [PrinContain]::Spawn((Ping-Line $LeakMark), [PrinContain]::CreateBreakawayFromJob)
            if ($broke -gt 0) { [PrinContain]::Kill($broke) | Out-Null; "probe: 46: a start broke away from the job: $broke"; exit 46 }
            if ($broke -ne -$BreakawayRefused) { "probe: 46: the breakaway start refused, not as a breakaway ($(-$broke)), at $active of $limit active"; exit 46 }
            # Off the probe's console, as `setsid` leaves its session; 0x08000000 is CREATE_NO_WINDOW.
            $left = [PrinContain]::Spawn((Ping-Line $LeakMark), 0x08000000)
            Start-Sleep -Seconds 1
            if ($left -le 0 -or -not (Get-Process -Id $left -ErrorAction Ignore)) { "probe: 46: the child to leave behind isn't running ($left)"; exit 46 }
            "probe: 46: the breakaway start refused ($(-$broke)); child $left left running"
            exit 0
        } catch {
            "probe: failed: $(Reason $_)"
            exit 40
        }
    }
    'hold' {
        # Run by `check`, inside the job: a tree three deep that would outlive this script, which waits to be killed.
        Load
        $a = [PrinContain]::Spawn((Ping-Line $LeakMark), 0)
        $b = [PrinContain]::Spawn('"' + (Join-Path $env:SystemRoot 'System32\cmd.exe') + '" /d /c ' + (Ping-Line $LeakMark), 0)
        "hold: started $a and $b"
        Start-Sleep -Seconds 600
        exit 0
    }
    'check' {
        if ($env:PRIN_CONTAIN -ne 'required') {
            'win_contain.ps1 check: containment not required here'
            exit 0
        }
        if ($env:OS -ne 'Windows_NT') { Fail 'containment required, and this is not Windows' }
        Load
        $shell = @('powershell', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath)
        $script:Outer = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\PING.EXE') -ArgumentList "-n 600 -w 1000 $OuterMark" -WindowStyle Hidden -PassThru
        $found = @(Get-Marked $OuterMark | Where-Object { $_.ProcessId -eq $Outer.Id })
        if (-not $found) { Fail "check: the control can't find its own process $($Outer.Id) by its marker" }
        $code = [PrinContain]::Commit([uint64]($MemoryCap + 65536))
        if ($code -ne 0) { Fail "check: the control can't commit more than the cap outside the job ($code): a refusal inside couldn't be told from the machine's own" }
        "win_contain.ps1 check: the control: outside process $($Outer.Id) found by its marker; $($MemoryCap + 65536) bytes committed outside the job"
        if (Get-Marked $LeakMark) { Fail "check: a process marked $LeakMark is already running" }

        $gaps = @()
        $modes = @(
            @{ Name = 'run'; Argv = $shell + 'run' + $shell + @('probe', $Outer.Id, $ProcessLimit, 1); Limit = $null }
            @{ Name = 'run nested in run'; Argv = $shell + 'run' + $shell + 'run' + $shell + @('probe', $Outer.Id, $ProcessLimit, 2); Limit = $null }
            @{ Name = "run at $CheckProcessLimit processes"; Argv = $shell + 'run' + $shell + @('probe', $Outer.Id, $CheckProcessLimit, 1); Limit = $CheckProcessLimit }
        )
        foreach ($mode in $modes) {
            $env:PRIN_CONTAIN_PROCESSES = $mode.Limit
            $command, $commandArgs = $mode.Argv
            $global:LASTEXITCODE = $null
            $lines = @(& $command @commandArgs)
            $code = $LASTEXITCODE
            Remove-Item Env:PRIN_CONTAIN_PROCESSES -ErrorAction Ignore
            $lines
            if ($code -ne 0) { Fail "check: inside ``$($mode.Name)``: exit $code" }
            if (-not (Get-Process -Id $Outer.Id -ErrorAction Ignore)) { Fail "check: the process outside died during ``$($mode.Name)``: the broadcast reached it" }
            $leak = Get-Marked $LeakMark
            if ($leak) {
                $leak | ForEach-Object { "left: $($_.ProcessId) $($_.Name) $($_.CommandLine)"; [PrinContain]::Kill([int]$_.ProcessId) | Out-Null }
                Fail "check: $($leak.Count) process(es) outlived ``$($mode.Name)``"
            }
            $gaps += @($lines | Where-Object { "$_".StartsWith('probe: 42 gap') })
            "win_contain.ps1 check: ``$($mode.Name)`` holds: the job's limits read back, the broadcast stayed inside, the cap held at its boundary, no breakaway, nothing left"
        }

        # Kill-on-close: this script's stand-in killed outright, with a tree running in its job.
        $argv = $shell + 'run' + $shell + 'hold'
        $wrapper = Start-Process -FilePath $argv[0] -ArgumentList (($argv[1..($argv.Count - 1)] | ForEach-Object { [PrinContain]::Quote([string]$_) }) -join ' ') -WindowStyle Hidden -PassThru
        $until = (Get-Date).AddSeconds(60)
        while (@(Get-Marked $LeakMark | Where-Object { $_.Name -eq 'PING.EXE' }).Count -lt 2) {
            if ((Get-Date) -gt $until) { Stop-Process -Id $wrapper.Id -Force -ErrorAction Ignore; Fail 'check: the held tree never started' }
            Start-Sleep -Milliseconds 250
        }
        $tree = @(Get-Marked $LeakMark)
        "win_contain.ps1 check: holding $($tree.Count) marked processes; killing the run's own process $($wrapper.Id)"
        Stop-Process -Id $wrapper.Id -Force
        $until = (Get-Date).AddSeconds(15)
        while (($leak = Get-Marked $LeakMark)) {
            if ((Get-Date) -gt $until) {
                $leak | ForEach-Object { "left: $($_.ProcessId) $($_.Name) $($_.CommandLine)"; [PrinContain]::Kill([int]$_.ProcessId) | Out-Null }
                Fail "check: $($leak.Count) process(es) outlived the job's last handle"
            }
            Start-Sleep -Milliseconds 250
        }
        "win_contain.ps1 check: kill-on-close holds: the run's process killed, its tree of $($tree.Count) gone with the job"

        $gaps | Select-Object -Unique
        Stop-Process -Id $Outer.Id -Force -ErrorAction Ignore
        'win_contain.ps1 check: contained: 41, 43, 44, 45 and 46, kill-on-close and nothing left hold in each mode.'
        'win_contain.ps1 check: not held, having no Job Object equivalent (RQ-751): 42, the outside process is seen and open to a kill by pid.'
        exit 0
    }
    default {
        Fail 'usage: win_contain.ps1 run <command> <arg>... | check'
    }
}
