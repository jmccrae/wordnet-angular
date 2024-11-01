<script setup>
//import HelloWorld from './components/HelloWorld.vue'
//import TheWelcome from './components/TheWelcome.vue'
import WordNet from './components/WordNet.vue'
</script>

<template>
  <div id="github-ribbon">
    <a href="https://github.com/globalwordnet/english-wordnet"><img width="149" height="149" src="https://github.blog/wp-content/uploads/2008/12/forkme_right_darkblue_121621.png?resize=149%2C149" class="attachment-full size-full" alt="Fork me on GitHub" data-recalc-dims="1"></a>
  </div>
  <div class="container">
    <div class="row" style="padding-top:5px;">
      <table width="100%">
        <tr>
          <td width="125px"><img src="./assets/english.svg"/></td>
          <td width="600px"><h1 class="en-title">Open English WordNet</h1></td>
          <td></td>
        </tr>
      </table>
    </div>
    <wordnet></wordnet>
    <footer class="footer">
      <div class="container">
        <div style="text-align:center;">
          <button type="button" class="btn download-btn"><a href="/static/english-wordnet-2023.xml.gz" target="_blank">Download as XML</a></button>
          <button type="button" class="btn download-btn"><a href="/static/english-wordnet-2023.ttl.gz" target="_blank">Download as RDF</a></button>
          <button type="button" class="btn download-btn"><a href="/static/english-wordnet-2023.zip" target="_blank">Download as WNDB</a></button>
        </div>
        <div style="text-align:right;" class="footer1">
          Open English WordNet is derived from <a href="http://wordnet.princeton.edu/">Princeton WordNet</a> by the Open English WordNet Community and released under the <a href="https://creativecommons.org/licenses/by/4.0/">Creative Commons Attribution (CC-BY) 4.0 License</a>. <a href="https://globalwordnet.github.io/gwadoc/">Further information about WordNet</a>. We welcome any corrections, improvements or other contributions at <a href="http://github.com/globalwordnet/english-wordnet">GitHub</a>. A full list of contributors is available on <a href="https://github.com/globalwordnet/english-wordnet/blob/master/README.md">GitHub</a>.</div>
        <div class="text-muted footer2" style="text-align:right;"> This interface was created by <a href="http://john.mccr.ae/">John P. McCrae</a> at the <a href="https://dsi.nuigalway.ie/">Data Science Institute</a>, <a href="http://www.universityofgalway.ie">University of Galway</a> (<a href="http://github.com/jmccrae/wordnet-angular">GitHub</a>). Development of this interface is supported by <a href="https://www.sfi.ie/">Science Foundation Ireland</a> as part of the <a href="https://www.insight-centre.org/">Insight Centre for Data Analytics</a> and the European Union's Horizon 2020 research and innovation programme under grant agreement No 731015 (<a href="http://elex.is/">ELEXIS</a>).
        </div>
      </div>
    </footer>
  </div>

</template>
