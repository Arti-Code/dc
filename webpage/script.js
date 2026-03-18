/* eslint-env browser */

let pc = new RTCPeerConnection({
  iceServers: [
    {
      urls: 'stun:stun.l.google.com:19302'
    },
    {
      urls: [
        "stun:fr-turn8.xirsys.com",
      ]
    },
    {
      username: "xrlEivlkdTCQvwPYbCRHDur872L9CNM7DlbAya3tEhbBcn7zMgFFN8q43pP_2v-4AAAAAGmxwT1nd296ZHlr",
      credential: "d05d03e4-1d7f-11f1-b1bb-be96737d4d7e",
      urls: [
        "turn:fr-turn8.xirsys.com:80?transport=udp",
        "turn:fr-turn8.xirsys.com:3478?transport=udp",
        "turn:fr-turn8.xirsys.com:80?transport=tcp",
        "turn:fr-turn8.xirsys.com:3478?transport=tcp",
        "turns:fr-turn8.xirsys.com:443?transport=tcp",
        "turns:fr-turn8.xirsys.com:5349?transport=tcp"
      ]
    }
  ]
})
let log = msg => {
  document.getElementById('div').innerHTML += msg + '<br>'
}

pc.ontrack = function (event) {
  var el = document.createElement(event.track.kind)
  el.srcObject = event.streams[0]
  el.autoplay = true
  el.controls = true

  document.getElementById('remoteVideos').appendChild(el)
}

pc.oniceconnectionstatechange = e => log(pc.iceConnectionState)
pc.onicecandidate = async event => {
  if (event.candidate === null) {
    let sdp = btoa(JSON.stringify(pc.localDescription))
    //const newHandle = await window.showSaveFilePicker();
    //const writableStream = await newHandle.createWritable();
    //await writableStream.write("This is my file content");
    //await writableStream.close();
    //let writable = createWritable();
    //let sdp8 = Uint8Array.from(sdp);
    //const file = new File(sdp8, "descr.sdp", {
    //    type: "text/plain",
    //});
    //writeFile("descr.sdp", "b", sdp)
    
    // Source - https://stackoverflow.com/a/24495213
// Posted by Endless, modified by community. See post 'Timeline' for change history
// Retrieved 2026-03-15, License - CC BY-SA 3.0

  var parts = [
    new Blob(['you construct a file...'], {type: 'text/plain'}),
    ' Same way as you do with blob',
    new Uint16Array([33])
  ];

  // Construct a file
  var file = new File(parts, 'sample.txt', {
      lastModified: new Date(0), // optional - default = now
      type: "overide/mimetype" // optional - default = ''
  });

  var fr = new FileReader();
  var fw = new FileSystem();
  
  fr.onload = function(evt){
     document.body.innerHTML = evt.target.result + "<br><a href="+URL.createObjectURL(file)+" download=" + file.name + ">Download " + file.name + "</a><br>type: "+file.type+"<br>last modified: "+ file.lastModifiedDate
  }

  fr.readAsText(file);

    document.getElementById('localSessionDescription').value = sdp
  }
}

// Offer to receive 1 audio, and 2 video tracks
pc.addTransceiver('audio', {'direction': 'recvonly'})
pc.addTransceiver('video', {'direction': 'recvonly'})
pc.addTransceiver('video', {'direction': 'recvonly'})
pc.createOffer().then(d => pc.setLocalDescription(d)).catch(log)

window.startSession = () => {
  let sd = document.getElementById('remoteSessionDescription').value
  if (sd === '') {
    return alert('Session Description must not be empty')
  }

  try {
    pc.setRemoteDescription(new RTCSessionDescription(JSON.parse(atob(sd))))
  } catch (e) {
    alert(e)
  }
}
