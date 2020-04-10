angular.module('app', ['ngMaterial'])
    .config(function($locationProvider) {
        $locationProvider.html5Mode(true);
    });


angular.module('app').component('wordnet', {
    templateUrl: 'static/wordnet.html',
    controller: function($scope) {
        $scope.synsets = [];
        $scope.focus = "";
        $scope.display = {
            show_wn31: true,
            display: false,
            langs: false,
            lang_en: true
        };
        $scope.selectAllPolyWN = function() {
            $scope.display.lang_en = true;
            $scope.display.lang_hr = true;
            $scope.display.lang_fr = true;
            $scope.display.lang_lv = true;
            $scope.display.lang_mt = true;
            $scope.display.lang_ga = true;
            $scope.display.lang_pt = true;
            $scope.display.lang_cs = true;
            $scope.display.lang_el = true;
            $scope.display.lang_it = true;
            $scope.display.lang_nl = true;
            $scope.display.lang_bg = true;
            $scope.display.lang_et = true;
            $scope.display.lang_de = true;
            $scope.display.lang_sv = true;
            $scope.display.lang_da = true;
            $scope.display.lang_sk = true;
            $scope.display.lang_es = true;
            $scope.display.lang_fi = true;
            $scope.display.lang_ro = true;
            $scope.display.lang_lt = true;
            $scope.display.lang_sl = true;
            $scope.display.lang_hu = true;
            $scope.display.lang_pl = true;
        };
    }
});


angular.module('app').controller('SearchController',
        function($scope, $http, $location) {
    var self = this;
    self.index = 'lemma-en';
    var m = $location.path().match("/(.*)/(.*)");
    self.selectedItemChange = function(item) {
        if(item) {
            $location.path("/"+ self.index + "/"+item.item);
            if(self.index.startsWith("lemma-")) {
                var lang = self.index.substring(6);
                // This is quite an ugly hack
                eval("$scope.display.lang_" + lang + " = true;");
            }
            $scope.$parent.focus = item.item;
            $http.get("/json/"+ self.index + "/"+item.item).then(
                function(result) {
                    $scope.$parent.synsets = result.data;
                    $scope.$parent.link = self.index + "/" + item.item;
                }, function(response) {
                    console.log(response.data);
                });
        } else {
            $scope.$parent.focus = "";
            $scope.$parent.synsets = [];
            $scope.$parent.link = "";
        }
    };

    if(m) {
        self.index = m[1];
        if(m[2].endsWith(".html")) {
            self.selectedItem = { item: m[2].substring(0, m[2].length - 5), display: m[2] };
        } else {
            self.selectedItem = { item: m[2], display: m[2] };
        }
        self.selectedItemChange(self.selectedItem);
    }
    self.querySearch = function (query, $timeout) {
       return $http.get("/autocomplete/"+ self.index + "/"+ query).then(
            function(result) {
                // See https://github.com/angular/material/issues/6668
                var outerContainer = document.querySelector('.md-autocomplete-suggestions-container');
                var innerContainer = document.querySelector('.md-virtual-repeat-sizer');
                outerContainer.style.height = innerContainer.style.height;
                return result.data;
            }, function(response) {
                console.log(response.data);
            });
        };
});

angular.module('app').component('synset', {
        templateUrl: '/static/synset.html',
        bindings: {
            synset: '=',
            display: '<',
            focus: '='
        },
        controller: function($http) {
            var ctrl = this;
            ctrl.targetsynsets = [];
            $http.get("/json_rel/" + this.synset.id).then(
                    function(response) {
                        ctrl.targetsynsets = response.data.slice(0,25);
                        ctrl.targetsynsetsextra = response.data.slice(25,response.data.length);
                    }, function(response) { /*alert(response);*/ }
            );
            ctrl.hasSubcats = function() {
                for(i = 0; i < this.synset.lemmas.length; i++) {
                    if(this.synset.lemmas[i].subcats.length > 0) {
                        return true;
                    }
                }
                return false;
            };
            ctrl.underlineSubcat = function(subcat, lemma) {
                return subcat.replace('----', '<span class="underline">' + lemma + '</span>');
            };
            ctrl.extendtargetsynsets = function() {
                ctrl.targetsynsets = ctrl.targetsynsets.concat(
                        ctrl.targetsynsetsextra.slice(0,25));
                ctrl.targetsynsetsextra = ctrl.targetsynsetsextra.slice(25,ctrl.targetsynsetsextra.length);
            };
        }
    });

angular.module('app').component('synset2', {
    templateUrl: '/static/synset.html',
    bindings: {
        synset: '<',
        display: '<',
        focus: '='
    },
    controller: function($http) {
        var ctrl = this;
        ctrl.targetsynsets = [];
        $http.get("/json_rel/" + this.synset.id).then(
                function(response) {
                    ctrl.targetsynsets = response.data.slice(0,50);
                    ctrl.targetsynsetsextra = response.data.slice(50,response.data.length);
                }, function(response) { /*alert(response);*/ }
        );
        ctrl.hasSubcats = function() {
            for(i = 0; i < this.synset.lemmas.length; i++) {
                if(this.synset.lemmas[i].subcats.length > 0) {
                    return true;
                }
            }
            return false;
        };
        ctrl.underlineSubcat = function(subcat, lemma) {
            return subcat.replace('----', '<span class="underline">' + lemma + '</span>');
        };
        ctrl.extendtargetsynsets = function() {
            ctrl.targetsynsets = ctrl.targetsynsets.concat(
                    ctrl.targetsynsetsextra.slice(0,25));
            ctrl.targetsynsetsextra = ctrl.targetsynsetsextra.slice(25,ctrl.targetsynsetsextra.length);
        };
 }
    //,
//    controller: function($http) {
//        var ctrl = this;
//        ctrl.synset = null;
//        for(i = 0; i < target_synsets.length; i++) {
//            if(ctrl.target === target_synsets[j].id) {
//                ctrl.synset = target_synsets[j];
//            }
//        }
//        if(ctrl.synset == null) {
//            $http.get("/json/id/" + ctrl.target).then(
//                function(response) {
//                    ctrl.synset = response.data[0];
//                },
//                function(response) {
//                    console.log(response.data);
//                });
//        }
//    }
});

angular.module('app').component('relation', {
        templateUrl: '/static/relation.html',
        bindings: {
            fullname: '@',
            relation: '@',
            relations: '=',
            display: '<',
            targetsynsets: '<'
        },
        controller: function($http) {
            var ctrl = this;
            this.show = false;
        }
//        controller: function($scope,$http) {
//            console.log("Loading synsets");
//            for(r = 0; r < this.relations.length; r++) {
//                var relation = this.relations[r];
//                console.log(relation);
//                $http.get("/json/id/" + relation.target).then(
//                    function(response) {
//                        console.log(response.data);
//                        relation.synset = response.data[0];
//                        console.log(relation);
//                    },
//                    function(response) {
//                        console.log(response.data);
//                    });
//            }
//        }

    });


angular.module('app').filter('isAdjective', function() {
    return function(items) {
        var filtered = [];
        for(var i = 0; i < items.length; i++) {
            var item = items[i];
            if(item.pos === 'a' || item.pos === 's') {
                filtered.push(item);
            }
        }
        return filtered;
    }
});

// From https://github.com/EricWVGG/AngularSlideables
angular.module('app')
.directive('slideable', function () {
    return {
        restrict:'C',
        compile: function (element, attr) {
            // wrap tag
            var contents = element.html();
            element.html('<div class="slideable_content" style="margin:0 !important; padding:0 !important" >' + contents + '</div>');

            return function postLink(scope, element, attrs) {
                // default properties
                attrs.duration = (!attrs.duration) ? '.3s' : attrs.duration;
                attrs.easing = (!attrs.easing) ? 'ease-in-out' : attrs.easing;
                element.css({
                    'overflow': 'hidden',
                    'height': '0px',
                    'transitionProperty': 'height',
                    'transitionDuration': attrs.duration,
                    'transitionTimingFunction': attrs.easing
                });
            };
        }
    };
})
.directive('slideToggle', function() {
    return {
        restrict: 'A',
        link: function(scope, element, attrs) {
            var target = document.querySelector(attrs.slideToggle);
            attrs.expanded = false;
            element.bind('click', function() {
                var content = target.querySelector('.slideable_content');
                if(!attrs.expanded) {
                    content.style.border = '1px solid rgba(0,0,0,0)';
                    var y = content.clientHeight;
                    content.style.border = 0;
                    target.style.height = y + 'px';
                } else {
                    target.style.height = '0px';
                }
                attrs.expanded = !attrs.expanded;
            });
        }
    }
});

angular.module('app').run(function($rootScope, $location){
   //Bind the `$locationChangeSuccess` event on the rootScope, so that we dont need to 
   //bind in induvidual controllers.

   $rootScope.$on('$locationChangeSuccess', function() {
        $rootScope.actualLocation = $location.path();
    });        

   $rootScope.$watch(function () {return $location.path()}, function (newLocation, oldLocation) {
        if($rootScope.actualLocation === newLocation) {
            location.reload();
        }
    });
});
