with Interfaces.C;

package body Spark is
   pragma SPARK_Mode (On);

   procedure Get_Version
     (Version_Out : out Interfaces.C.char_array;
      Version_Len : out Interfaces.C.size_t)
   is
      Version_Str : constant String := SPARK_Version;
   begin
      -- Initialize output
      Version_Out := (others => Interfaces.C.char'Val (0));
      Version_Len := 0;

      -- Copy version string (bounded copy)
      for I in Version_Str'Range loop
         exit when I > Version_Out'Length;
         Version_Out (Interfaces.C.size_t (I)) :=
            Interfaces.C.char'Val (Character'Pos (Version_Str (I)));
         Version_Len := Interfaces.C.size_t (I);
      end loop;
   end Get_Version;

end Spark;
