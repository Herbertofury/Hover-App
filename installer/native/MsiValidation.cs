// Native Windows SDK Evalcom2 ABI, documented in evalcom2.h. No COM registration.
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
namespace HoverPackaging {
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    public delegate bool Display(IntPtr context,int type,[MarshalAs(UnmanagedType.LPWStr)]string ice,[MarshalAs(UnmanagedType.LPWStr)]string description,[MarshalAs(UnmanagedType.LPWStr)]string location);
    [UnmanagedFunctionPointer(CallingConvention.StdCall)]
    public delegate bool Status(int status,[MarshalAs(UnmanagedType.LPWStr)]string data,IntPtr context);
    [ComImport,Guid("E482E5C6-E31E-4143-A2E6-DBC3D8E4B8D3"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IValidate {
        void OpenDatabase([MarshalAs(UnmanagedType.LPWStr)]string path);
        void OpenCUB([MarshalAs(UnmanagedType.LPWStr)]string path);
        void CloseDatabase(); void CloseCUB();
        void SetDisplay([MarshalAs(UnmanagedType.FunctionPtr)]Display callback,IntPtr context);
        void SetStatus([MarshalAs(UnmanagedType.FunctionPtr)]Status callback,IntPtr context);
        [PreserveSig] int Validate([MarshalAs(UnmanagedType.LPWStr)]string ices);
    }
    [ComImport,Guid("00000001-0000-0000-C000-000000000046"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IClassFactory {
        void CreateInstance(IntPtr outer,ref Guid iid,out IntPtr result);
        void LockServer([MarshalAs(UnmanagedType.Bool)]bool locked);
    }
    public class Message { public int Type; public string Ice; public string Description; public string Location; }
    public class Result { public int SchemaVersion=1; public string Scope="Microsoft SDK darice.cub, all ICEs"; public int HResult; public int Errors; public int Warnings; public bool Completed; public List<Message> Messages=new List<Message>(); public List<string> Status=new List<string>(); }
    public static class MsiValidation {
        [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)]static extern IntPtr LoadLibraryEx(string path,IntPtr file,uint flags);
        [DllImport("kernel32.dll",CharSet=CharSet.Ansi,ExactSpelling=true)]static extern IntPtr GetProcAddress(IntPtr module,string name);
        [DllImport("kernel32.dll")]static extern bool FreeLibrary(IntPtr module);
        [UnmanagedFunctionPointer(CallingConvention.StdCall)]delegate int GetClass(ref Guid clsid,ref Guid iid,out IntPtr result);
        public static Result Run(string engine,string package,string cube){
            var result=new Result(); IValidate validator=null; IClassFactory factory=null;
            var module=LoadLibraryEx(engine,IntPtr.Zero,0x100|0x800);
            if(module==IntPtr.Zero)throw new System.ComponentModel.Win32Exception();
            try {
                var clsid=new Guid("6E5E1910-8053-4660-B795-6B612E29BC58");var factoryId=typeof(IClassFactory).GUID;IntPtr pointer;
                var entry=GetProcAddress(module,"DllGetClassObject");if(entry==IntPtr.Zero)throw new Exception("SDK has no class factory");
                var get=(GetClass)Marshal.GetDelegateForFunctionPointer(entry,typeof(GetClass));
                Marshal.ThrowExceptionForHR(get(ref clsid,ref factoryId,out pointer));
                factory=(IClassFactory)Marshal.GetObjectForIUnknown(pointer);Marshal.Release(pointer);
                var iid=typeof(IValidate).GUID;factory.CreateInstance(IntPtr.Zero,ref iid,out pointer);
                validator=(IValidate)Marshal.GetObjectForIUnknown(pointer);Marshal.Release(pointer);
                Display display=(context,type,ice,description,location)=>{result.Messages.Add(new Message{Type=type,Ice=ice,Description=description,Location=location}); if(type==1)result.Errors++; if(type==2)result.Warnings++;return true;};
                Status status=(code,data,context)=>{result.Status.Add(code+":"+data);if(code==8)result.Completed=true;return true;};
                validator.OpenDatabase(package);validator.OpenCUB(cube);validator.SetDisplay(display,IntPtr.Zero);validator.SetStatus(status,IntPtr.Zero);
                result.HResult=validator.Validate(null);GC.KeepAlive(display);GC.KeepAlive(status);
                validator.CloseCUB();validator.CloseDatabase();return result;
            } finally {if(validator!=null)Marshal.FinalReleaseComObject(validator);if(factory!=null)Marshal.FinalReleaseComObject(factory);FreeLibrary(module);}
        }
    }
}
