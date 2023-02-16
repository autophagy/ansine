function refresh_metrics(second_interval) {
   const cpu_usage_label = document.getElementById('cpu-usage-label');
   const mem_usage_label = document.getElementById('mem-usage-label');
   const swap_usage_label = document.getElementById('swap-usage-label');

   const uptime = document.getElementById('uptime');
   const cpu_usage = document.getElementById('cpu-usage');
   const mem_usage = document.getElementById('mem-usage');
   const swap_usage = document.getElementById('swap-usage');

   setInterval(function() {
      fetch('metrics')
         .then((response) => response.json())
         .then((data) => {
            uptime.textContent = data.uptime;

            cpu_usage_label.textContent = `CPU ${data.cpu}%`;
            cpu_usage.value = data.cpu;

            mem_usage_label.textContent = `MEM ${data.mem}%`;
            mem_usage.value = data.mem;

            swap_usage_label.textContent = `SWP ${data.swap}%`;
            swap_usage.value = data.swap;
         });
   }, second_interval * 1000);
}
